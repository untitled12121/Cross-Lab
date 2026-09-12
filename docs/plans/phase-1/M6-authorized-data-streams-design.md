# M6 Authorized Data Streams — Design

**Status:** Draft for written review  
**Milestone:** Phase 1 / M6  
**Baseline:** canonical `main` after M5 final integration (`c24f68342709efc72549f0bf8f3ea8df2ef6de48`)

## 1. Goal

M6 proves that bulk payload bytes can flow only through a bounded data stream backed by a currently valid local `AuthorizedOperation`.

The milestone extends the deterministic in-memory simulator. It does not add real networking, filesystem transfer, resumability, UI, persistence, platform code, or M7 reconnect/revocation lifecycle work.

The governing rule remains:

```text
control authorizes
      |
      v
AuthorizedOperation
      |
      v
DataStreamOpen + local admission
      |
      v
bounded payload delivery
```

A transport stream, `StreamId`, or `OperationId` is never sufficient authority by itself.

## 2. Source Contracts

M6 preserves the existing architecture:

- the Master Architecture separates control-plane authorization from data-plane payloads and requires active operation binding;
- `SESSION-TRANSPORT.md` requires bounded header parsing, exact session/peer/operation binding, operation-state/revision checks, atomic stream-use reservation, and no payload exposure before admission;
- `CORE-SIMULATOR.md` requires S-007 plus N-035..N-039 and bounded stream/backpressure/cancellation behavior;
- M3 `AuthorizedOperation` remains the owner of operation authority;
- M4 `DataStreamOpen` and its strict bounded protobuf decoder remain the wire contract;
- M5 `LogicalSession`, `SessionContext`, `TransportConnection`, and `MemoryTransportPair` remain the session/transport foundation.

This design does not introduce a new authority model or transport-specific domain type.

## 3. Ownership Boundaries

### `crosslab-policy`

Owns operation authority and stream-use budget semantics.

It will continue to own:

- operation binding to source/destination devices;
- `SessionId`;
- capability/version/operation;
- trust and policy revision snapshots;
- expiry and terminal state;
- use policy and reserved stream-use count.

It does not parse stream headers or own transport handles.

### `crosslab-protocol`

Owns the existing `DataStreamOpen` wire/domain representation and bounded framing/decoding.

M6 should not change the wire schema unless implementation discovers a concrete missing field. The current fields are sufficient for the approved design.

### `crosslab-core`

Owns session-bound stream admission and bounded local operation/stream tracking.

Core validates the decoded open request against the active session and locally held operation before the simulator may deliver payload bytes.

### `crosslab-sim`

Owns deterministic bounded in-memory stream queues and scenario composition only.

It does not own authorization rules or duplicate operation/admission semantics.

## 4. Bounded Use Policies

`UsePolicy` gains one bounded multi-stream form:

```text
SingleAction
SingleStream
MultiStream { max_streams: NonZeroU32 }
```

There is no unlimited/reusable stream policy in M6.

Semantics:

- `SingleAction` cannot admit a data stream;
- `SingleStream` permits exactly one reserved stream use and requires `stream_index == 0`;
- `MultiStream { max_streams }` permits indices in `0..max_streams`;
- indices need not arrive in order;
- an index can be reserved only once;
- a successfully admitted stream permanently spends its stream-use slot;
- cancellation after admission does not refund that slot.

`AuthorizedOperation` tracks the count of successfully reserved stream uses. Reservation validates the existing operation binding/expiry/revisions before incrementing the count.

Reservation does **not** immediately change the operation to `Consumed`. This allows cancellation/revocation to remain meaningful while an admitted stream is active. Core marks an operation consumed when its allowed stream budget is exhausted and all admitted streams using that operation are terminal. If fewer than the maximum multi-stream uses are opened, the operation remains active until explicit cancellation, revocation, expiry, or later completion logic.

## 5. Session-Bound Stream Admission

Core adds a focused stream-admission feature separate from `ControlDispatcher`.

Conceptually:

```text
encoded stream open
        |
        v
strict protocol decode
        |
        v
StreamAdmission
  active session/context
  exact SessionId
  negotiated capability/version
  unique StreamId
  known local OperationId
  source/destination peer binding
  operation/capability/version/name match
  direction/opening-peer match
  stream_index valid + unused
  operation active/not expired
  trust revision unchanged
  policy revision unchanged
  stream-use budget available
        |
        v
atomic reservation
        |
        v
AdmittedStream
        |
        v
payload may be exposed
```

### Inbound direction binding

For an inbound stream from the authenticated peer:

- `SourceToDestination` means the authenticated peer is the operation source and the local device is the destination;
- `DestinationToSource` means the local device is the operation source and the authenticated peer is the destination.

The resulting `OperationUseContext` is passed through the existing `AuthorizedOperation` validation path. A direction that does not match the operation's source/destination binding is rejected.

### Atomicity

Admission uses one exclusive mutable core state transition. The operation budget and the used `StreamId` / `(OperationId, stream_index)` records are committed only after every validation succeeds.

No partially admitted state is left behind after a rejected open.

## 6. Bounded Operation and Replay State

Each logical session owns a bounded stream-admission registry containing:

- locally issued/accepted `AuthorizedOperation` records available for stream use;
- used `StreamId` history;
- used `(OperationId, stream_index)` history;
- active admitted stream records.

The registry never silently evicts security-relevant replay state. If its configured capacity is exhausted, new registration/admission returns a typed resource-limit error rather than growing without bound or forgetting prior use.

An operation registered into a per-session registry must belong to that session. Wrong-session operations are rejected at registration or admission and can never authorize payload.

## 7. Admitted Stream Lifecycle

Core tracks admitted streams with explicit lifecycle state sufficient for M6:

```text
Admitted
  -> Finished
  -> terminal

Admitted
  -> Cancelled
  -> terminal
```

Session shutdown/cancellation makes all active admitted streams terminal and cancels active registered operations as appropriate.

Stream-use slots remain spent after admission regardless of graceful finish or stream-local cancellation. This avoids authority being recreated by setup races or repeated cancellation/retry.

For a stream-budget-exhausted operation, once no admitted stream for that operation remains active, core transitions the still-active operation to `Consumed`.

A previously `Cancelled`, `Expired`, `Revoked`, or `Consumed` operation cannot admit another stream.

## 8. Transport-Neutral Data Stream Seam

M6 extends the existing transport-neutral seam rather than adding simulator types to core domain state.

The semantic surface is:

```text
TransportConnection
  ... existing control methods ...
  try_open_uni_stream(opening_frame)
  try_accept_uni_stream()

TransportSendStream
  try_send_chunk(chunk)
  finish()
  cancel()

TransportReceiveStream
  try_receive_chunk()
  cancel()
```

Exact Rust naming/ownership may be refined during implementation, but these responsibilities are fixed.

Important rules:

- the transport carries opaque opening bytes and never interprets `DataStreamOpen`;
- opening bytes are decoded by `crosslab-protocol` before core admission;
- stream handle types are transport-neutral traits/owned wrappers, never Quinn/Iroh/libp2p types;
- send/open errors retain unsent owned bytes when useful for retry/backpressure handling;
- error `Debug` output must not dump payload bytes;
- no general async runtime or polling loop is introduced in M6.

The split between graceful `finish` and abortive `cancel` intentionally maps to common real-stream semantics without importing a concrete transport API.

## 9. In-Memory Stream Transport

`MemoryTransportPair` gains deterministic unidirectional streams with explicit configuration for:

- bounded pending stream opens;
- bounded queued chunks per stream;
- maximum chunk bytes;
- existing bounded control capacity;
- existing connection/channel-binding behavior.

Existing `MemoryTransportPair::new(...)` compatibility should be preserved where practical; stream-focused tests may use an explicit configuration constructor.

Opening a stream:

1. validates simulator resource limits, not protocol authority;
2. creates one shared bounded stream state;
3. returns a sender handle to the opener;
4. queues an incoming opening frame + receiver handle for the peer.

If the pending-open queue is full, opening fails without creating a dangling stream.

### Payload backpressure

`try_send_chunk` rejects a chunk that exceeds the configured maximum and returns `Full` when the bounded chunk queue has no capacity. The unsent chunk remains owned by the caller through the typed error.

No unbounded byte accumulation is permitted.

### Close behavior

- `finish()` stops further sends but allows already queued chunks to drain before the receiver observes graceful closure;
- `cancel()` aborts the stream, clears queued chunks, and makes both sides observe cancellation/closure deterministically;
- connection close cancels pending and active data streams and abandons bounded queued payload;
- repeated finish/cancel/connection-close operations are idempotent where meaningful.

## 10. Simulator Composition

Stream-specific simulator behavior stays feature-focused rather than bloating `SimNode`.

A small simulator stream runtime composes:

- the active `LogicalSession` / `SessionContext`;
- core `StreamAdmission`;
- the transport-neutral data-stream methods;
- strict `encode_data_stream_open` / `decode_data_stream_open`;
- bounded active receive handles.

`SimNode` may delegate small public convenience methods to this feature, but authorization/admission and byte-queue logic remain in their owning modules.

The M6 end-to-end scenario is synthetic `files.transfer/send`; it does not read or write the real filesystem.

## 11. Failure Semantics

M6 returns typed failures for at least:

- inactive/wrong session;
- operation not found;
- wrong source/destination peer binding;
- unsupported/not-negotiated capability or version;
- capability/operation/direction mismatch;
- invalid/duplicate/out-of-range stream index;
- duplicate `StreamId`;
- `SingleAction` used for a stream;
- stream-use budget exhausted;
- expired/cancelled/revoked/consumed operation;
- trust or policy revision change;
- bounded operation/admission/open/chunk resource exhaustion;
- malformed/oversized `DataStreamOpen` frame;
- stream/connection cancellation or closure.

A rejected stream open is never exposed to the capability handler. M6 does not automatically close the entire logical session for every stream-local admission failure; M7 owns broader failure-lifecycle policy where required.

## 12. Testing Strategy

### Policy tests

Prove:

- `SingleAction` cannot reserve a stream;
- `SingleStream` reserves one use only;
- bounded multi-stream accepts unique indices only within its limit;
- reservation is denied after terminal state/expiry/revision change;
- spent slots are not refunded after cancellation;
- consumption occurs only after exhausted budget has no active admitted streams.

### Core tests

Prove N-035..N-039-style admission failures:

- missing `OperationId`;
- wrong session/peer/capability/version/operation/direction;
- expired/cancelled/revoked/consumed operation;
- duplicate `StreamId` or stream index;
- out-of-range index;
- policy/trust revision change;
- bounded registry exhaustion;
- no payload-visible admission token on failure.

### Memory transport tests

Prove:

- bounded pending-open ordering;
- open-queue saturation;
- ordered bounded chunk delivery;
- chunk-size rejection;
- chunk-queue backpressure with ownership preservation;
- graceful finish/drain;
- cancellation and connection-close propagation;
- no dangling stream on failed open.

### End-to-end simulator test

Prove S-007:

1. policy authorization produces a `files.transfer/send` grant;
2. an `AuthorizedOperation` is issued with `SingleStream`;
3. the receiver registers the operation;
4. the sender opens a memory data stream carrying encoded `DataStreamOpen`;
5. receiver decodes and admits it before exposing payload;
6. bounded synthetic chunks arrive in order;
7. stream finishes and operation becomes consumed;
8. a second stream attempt with the same single-stream operation is rejected.

Also prove cancellation during stream setup and deterministic shutdown with active streams to the extent owned by M6; full reconnect/revocation lifecycle remains M7.

The existing `data_stream_open` fuzz target remains the parser fuzz surface unless implementation introduces a genuinely new untrusted parser.

## 13. Scope Exclusions

M6 explicitly excludes:

- Quinn, Iroh, libp2p, sockets, TLS/rustls integration, relays, NAT traversal;
- real files, resume manifests, chunk hashing, deduplication, sync semantics;
- bidirectional application protocols beyond what is necessary to prove directional stream admission;
- unbounded stream authority;
- UI, desktop/mobile adapters, persistence, plugins, privileged services;
- M7 reconnect, replay-across-session, active revocation orchestration, and full runtime race testing;
- background async task systems.

## 14. Research Reuse Guidance

No external repository architecture is adopted for M6.

The uploaded Quinn reference confirms a useful future-compatible semantic distinction between graceful send completion and abort/reset, plus receive-side stop/cancellation. M6 mirrors only those generic lifecycle concepts behind Cross-Lab-owned transport-neutral interfaces. Quinn types and async runtime choices remain deferred to M8.

Existing Cross-Lab control backpressure/error patterns should be reused before introducing new abstractions.

## 15. Acceptance Boundary

M6 is ready to close only when:

- operation-bound stream admission is locally authoritative and fail closed;
- single-stream and bounded multi-stream budgets are enforced;
- payload is inaccessible before successful admission;
- all stream/open/chunk queues are explicitly bounded;
- backpressure preserves unsent payload ownership;
- cancellation/shutdown terminate simulator stream state deterministically;
- S-007 and applicable N-035..N-039/N-043/N-045 behavior pass;
- existing protocol fuzz smoke remains green;
- full workspace format/check/Clippy/tests are green;
- the final diff contains no real networking or M7 scope creep.
