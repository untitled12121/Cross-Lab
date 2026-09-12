# M8 Quinn Transport — Design

**Status:** Proposed for review; no M8 production code may begin until this design is approved and the implementation plan is committed.  
**Milestone:** Phase 1 / M8 — Quinn Transport  
**Baseline:** verified canonical `main` documentation checkpoint `b2e16f27a5c6d8307c58f4c6a4c760bb867ff223`  
**Primary contracts:** `docs/architecture/MASTER-ARCHITECTURE.md`, `docs/architecture/SESSION-TRANSPORT.md`, `docs/architecture/CORE-SIMULATOR.md`, M5–M7 implementation seams

## 1. Goal

M8 replaces only the deterministic link implementation with the first real encrypted IP transport while preserving Cross-Lab identity, trust, policy, session, control, operation, stream-admission, reconnect, revocation, and shutdown semantics.

The milestone proves the same domain behavior over a Quinn/QUIC connection:

```text
Cross-Lab identity / trust / policy
             |
      LogicalSession
             |
 control + authorized data semantics
             |
     TransportConnection
             |
   Quinn adapter (M8)
             |
       QUIC + TLS 1.3
             |
          UDP/IP
```

M8 is successful when real loopback/local QUIC connections can authenticate Cross-Lab peers using the existing session-auth protocol, bind that authentication to the protected QUIC connection, carry ordinary control and authorized data streams with explicit bounds/backpressure, fail closed on connection loss/revocation, and reconnect only through a fresh session.

M8 does not add discovery, NAT traversal, relay, Iroh, libp2p, route scoring, transport migration, session tickets, 0-RTT authorization, persistence, UI, platform adapters, privileged services, datagram consumers, or remote-network orchestration.

## 2. Verified Baseline

The verified `main` checkpoint already provides the domain seams M8 needs:

- `crosslab-core::TransportConnection` exposes transport-neutral security class, channel binding, metadata, bounded nonblocking control delivery, unidirectional data streams, close, and closed state;
- `LogicalSession` authenticates credentials/trust/protocol/features against an opaque `ChannelBinding` and records only the semantic `TransportSecurityClass`;
- session-auth hello/proof messages already have bounded wire codecs in `crosslab-protocol`;
- `SimNode` owns ordinary control dispatch and maps transport close/loss into the M7 terminal lifecycle;
- `SimStreamRuntime` owns authorized stream admission and cancels session-scoped operation authority on transport loss, revocation, or shutdown;
- reconnect already means a new transport binding, fresh nonces/proofs, a new `SessionId`, reset sequence state, capability renegotiation, and fresh authorization.

M8 therefore does not create a second session architecture and does not move Quinn types into core.

## 3. Research and Dependency Decision

### 3.1 Quinn

Use Quinn as the first real IP transport implementation, as already selected by the Master Architecture.

The M8 dependency baseline is:

```text
quinn  = 0.11.11
tokio  = 1.53.1
rustls = 0.23.x through Quinn unless a direct API requirement appears
rcgen  = 0.14.10 as a dev dependency for loopback certificate fixtures
```

`quinn` is MIT OR Apache-2.0, matching the Cross-Lab repository license.

Use Quinn with default features disabled and only the required runtime/TLS provider features enabled:

```toml
quinn = { version = "=0.11.11", default-features = false, features = ["runtime-tokio", "rustls-ring"] }
```

Do not enable platform certificate verification, qlog, alternate runtimes, datagram-specific application code, or other optional facilities without an M8 consumer.

Tokio is the one async runtime for this adapter. Cross-Lab core remains runtime-neutral.

### 3.2 Uploaded Quinn reference

The uploaded Quinn source archive is the preferred research reference, but the current execution environment could not reliably enumerate that archive. The design was therefore cross-checked against the maintained upstream Quinn 0.11.11 API/source documentation. Before production implementation, the implementation branch should inspect the uploaded archive in a normal local checkout if available and reconcile any relevant API/test patterns with the version pinned here.

### 3.3 Iroh and rust-libp2p

Do not introduce either dependency in M8. Their role remains M9 research for Internet/NAT/relay architecture. They may be consulted only to compare abstraction boundaries.

## 4. Crate and Dependency Boundary

Add one concrete adapter crate:

```text
transports/
└── quic/
    ├── Cargo.toml
    └── src/
```

Package name:

```text
crosslab-transport-quic
```

Dependency direction:

```text
crosslab-core  <── crosslab-transport-quic
                     |
                     +── quinn
                     +── tokio
```

Forbidden dependency direction:

```text
crosslab-core      -> quinn
crosslab-protocol  -> quinn
crosslab-policy    -> quinn
crosslab-identity  -> quinn
```

`quinn::Endpoint`, `quinn::Connection`, `quinn::SendStream`, `quinn::RecvStream`, rustls configuration objects, socket/runtime details, and TLS certificate types remain private to `crosslab-transport-quic`.

A dedicated abstract transport crate is still unnecessary because the semantic contract already lives narrowly in `crosslab-core` and is consumed by both the simulator and the concrete Quinn adapter.

## 5. Runtime Ownership

M8 does not create one Tokio runtime per connection.

The Quinn adapter is constructed from async application/test code running on a caller-owned Tokio runtime. The concrete adapter owns only connection-scoped driver tasks and handles required to bridge Quinn's async APIs into the existing bounded nonblocking `TransportConnection` contract.

Conceptually:

```text
caller-owned Tokio runtime
        |
        +-- Quinn Endpoint / connection
        |
        +-- connection driver tasks
                |
                +-- control send/receive bridge
                +-- uni-stream accept bridge
                +-- per-stream bounded bridge tasks
                +-- connection-loss monitor
```

Every spawned task is connection-owned. Adapter shutdown cancels/terminates those tasks and provides an async join/closed path to tests and future agents. No detached task may retain authority after connection close.

## 6. TLS Role Versus Cross-Lab Identity

QUIC/TLS provides confidentiality, integrity, and the cryptographic session used for channel binding. It does not define Cross-Lab device identity.

For M8 loopback integration tests:

- generate an ephemeral self-signed server certificate with `rcgen`;
- explicitly trust that certificate in the test client configuration;
- use the certificate only to establish a protected QUIC/TLS channel;
- perform Cross-Lab owner/device authentication separately over that channel using existing `SessionAuthHello` and `SessionAuthProofMessage` messages.

M8 does not define production certificate provisioning, certificate persistence, owner PKI, public CA dependence, or TLS certificate to `DeviceId` mapping. Those would be separate operational decisions if later needed.

## 7. Quinn Channel-Binding Profile

M8 resolves the deferred channel-binding mechanism using the TLS exporter exposed by Quinn's established `Connection`.

Proposed profile:

```text
profile_id = "quic-tls-exporter-v1"
output_len = 32 bytes
label      = "EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1"
context    = "crosslab.quic.transport.v1"
```

The adapter calls Quinn `Connection::export_keying_material` after the full handshake succeeds. Both endpoints use the exact same label, context, and output length.

Properties:

- the bytes come from TLS session secrets, not peer application data;
- both peers obtain matching binding bytes for the same connection;
- a fresh protected connection yields fresh exporter material;
- Cross-Lab session proofs already hash the binding profile/value into the authentication transcript;
- TLS transport identity does not become Cross-Lab identity authority.

M8 must not call `Connecting::into_0rtt` and must not expose or accept 0-RTT application/session authority. A connection becomes eligible for Cross-Lab authentication only after the full Quinn handshake completes and exporter material is available.

Because this profile affects security and cross-version compatibility, the first implementation task must record it in proposed `ADR-0008-quinn-channel-binding-profile-v1.md`. The ADR becomes accepted only with the M8 design approval; changing label/context/output semantics later requires an ADR update/supersession.

## 8. One QUIC Connection, Separate Logical Traffic Roles

M8 uses one QUIC connection per logical-session bootstrap attempt and separates traffic by QUIC stream role rather than by a second protocol stack.

### Control/bootstrap stream

The logical-session initiator opens one QUIC bidirectional stream. The responder accepts that stream. It is the only Phase 1 control stream for that connection.

Before `LogicalSession::Active`, it carries the existing bounded session-auth bootstrap frames:

```text
initiator -> SessionAuthHello
responder -> SessionAuthHello
initiator -> SessionAuthProofMessage
responder -> SessionAuthProofMessage
```

The orchestration may order proof exchange symmetrically as long as both sides build the same approved transcript and role-separated proofs. No ordinary capability/control envelope is accepted until both proofs verify and `LogicalSession` is active.

After activation, the same ordered reliable bidirectional QUIC stream carries ordinary encoded `ControlEnvelope` frames consumed by the existing `SimNode`/`ControlDispatcher` path.

### Data streams

Each Cross-Lab unidirectional data stream maps to one Quinn unidirectional stream.

The first record is the existing bounded encoded `DataStreamOpen` frame. Only after the receiver decodes it and `StreamAdmission` validates the active `SessionId`, `OperationId`, capability/version/direction/use policy, current trust/policy revision, and remaining stream use may payload chunks reach the capability-facing runtime.

No bidirectional application data-stream API is added in M8 because the current authorized-data contract has no consumer for it.

### Datagrams

Datagrams remain unused in M8. Quinn support does not justify adding a domain API without a concrete consumer.

## 9. Transport-Private Record Framing

QUIC streams are byte streams, while the current `TransportConnection` seam exchanges complete `Vec<u8>` frames/chunks. The Quinn adapter therefore owns a small private record framing layer; this is not a Cross-Lab public wire protocol.

Use a 4-byte unsigned big-endian length prefix followed by exactly that many bytes:

```text
u32_be length
[length] bytes
```

The adapter applies separate configured maximums for:

- session/bootstrap/control record bytes;
- data-stream opening-frame bytes;
- data chunk bytes.

The receiver validates the length before allocating the record buffer. Zero-length records are rejected for control/bootstrap/opening frames. Data chunk zero-length behavior follows the current stream contract and should be rejected unless an existing test proves it is meaningful.

A truncated prefix, truncated body, over-limit length, QUIC reset, or connection error terminates the affected stream; bootstrap/control framing failure is fatal to the logical session/connection.

This framing remains private to `crosslab-transport-quic`; protocol crates continue to own the inner message encoding and bounds.

## 10. Preserving the Existing Nonblocking Transport Contract

Quinn is async, while core intentionally exposes bounded nonblocking `try_*` operations. M8 bridges these models with bounded Tokio channels and connection-owned tasks.

### Control outbound

`try_send_control(frame)`:

1. reject if connection/control bridge is terminal;
2. reject an over-limit frame without consuming ownership;
3. `try_send` the frame into a bounded Tokio MPSC queue;
4. return `Full(frame)` if the local queue is saturated;
5. return `Closed(frame)` if terminal;
6. a single async writer task serializes queued records onto the control `SendStream`.

This preserves deterministic local backpressure while QUIC independently applies network flow control beneath the bridge.

### Control inbound

A single async reader task reads one bounded record at a time and awaits capacity in a bounded inbound queue. Because it awaits queue capacity instead of continuing to read, application backpressure propagates into QUIC receive flow control.

`try_receive_control()` pops from that bounded queue and returns `Empty` or `Closed` using the existing core semantics.

### Outbound uni stream

`try_open_uni_stream(opening_frame)` is nonblocking at the core seam. It reserves one bounded local outgoing-stream slot and returns a transport send handle whose chunk queue is also bounded.

A connection-owned async task then:

1. awaits Quinn `open_uni()`;
2. writes the bounded opening-frame record;
3. drains bounded chunk records in order;
4. calls Quinn `finish()` on normal completion;
5. calls Quinn `reset()` on cancellation/terminal error.

If the connection becomes terminal before opening completes, the handle transitions to closed/cancelled state and later calls fail closed.

### Inbound uni stream

A connection accept task awaits Quinn `accept_uni()`, bounded by configured stream concurrency. For each accepted stream it:

1. reads and validates the opening-frame record;
2. creates a bounded chunk bridge;
3. publishes one `IncomingUniStream` into the bounded inbound-stream queue;
4. reads subsequent bounded chunk records only as the local chunk bridge has capacity;
5. maps peer FIN to `Finished` and reset/stop/connection loss to cancellation/closed semantics.

Dropping/cancelling the receive handle calls Quinn `RecvStream::stop()` through the owning task/state; cancelling the send handle maps to `SendStream::reset()`.

## 11. Small Core Contract Tightening

Real network adapters must reject oversized outbound buffers before queuing them. The in-memory adapter currently has `TooLarge` only for data chunks.

M8 should add ownership-preserving variants:

```rust
ControlSendError::TooLarge(Vec<u8>)
StreamOpenError::TooLarge(Vec<u8>)
```

The memory transport gains matching configurable limits/tests so both transports preserve the same semantic contract. Existing `Full` and `Closed` behavior remains unchanged.

This is a transport-neutral strengthening of an existing bound, not a Quinn-specific domain leak. It requires no new crate dependency or session architecture.

No async methods, Tokio types, Quinn errors, TLS errors, socket addresses, or runtime handles are added to `crosslab-core` public transport traits.

## 12. Configuration and Resource Bounds

`crosslab-transport-quic` owns a typed `QuicTransportConfig` with nonzero bounded values for at least:

```text
control_queue_capacity
incoming_stream_queue_capacity
outgoing_stream_capacity
stream_chunk_queue_capacity
max_control_frame_bytes
max_opening_frame_bytes
max_chunk_bytes
max_concurrent_remote_uni_streams
max_concurrent_remote_bi_streams
stream_receive_window
connection_receive_window
idle_timeout
```

Initial defaults should be conservative and derived from existing protocol/frame bounds where possible rather than chosen as effectively unlimited values.

The adapter configures Quinn's concurrent stream and receive-window limits explicitly. Queue capacity and window size are separate controls: queue bounds limit local application memory, while QUIC windows limit transport buffering/flow-control exposure.

M8 must not use unbounded MPSC channels, unlimited `read_to_end`, infinite stream concurrency, infinite idle timeout, or a busy polling loop.

## 13. Connection State and Failure Mapping

The adapter owns one shared terminal connection state. Quinn connection closure, endpoint failure, fatal control framing/write failure, bootstrap failure, or explicit local close marks the adapter terminal once.

Terminal transition:

```text
mark closed
  -> stop accepting new control/stream work
  -> close local bridge senders
  -> cancel/reset active stream drivers
  -> Quinn Connection::close when locally initiated
  -> wake/drop blocked driver tasks
  -> allow caller to await task completion
```

After terminal transition:

- `try_send_control` returns `Closed(frame)`;
- `try_receive_control` drains no stale authority and returns `Closed` once no already-admitted record is intentionally retained;
- stream open/accept returns `Closed`;
- active stream handles become cancelled/closed;
- `is_closed()` is true.

The existing M7 runtimes remain responsible for translating those semantic `Closed` results into `LogicalSession::transport_lost()` and session-scoped authority cancellation.

Transport errors never mutate trust or policy directly.

## 14. Session Authentication Over QUIC

M8 adds an integration orchestration layer outside domain crates to prove that the existing authentication protocol actually crosses the protected connection.

The flow is:

```text
1. establish full Quinn/TLS connection
2. derive Quinn exporter ChannelBinding
3. establish reserved control/bootstrap bi stream
4. exchange bounded SessionAuthHello wire messages
5. locally validate credentials/trust and negotiate protocol/features
6. build identical SessionAuthTranscriptV1 including Quinn binding
7. exchange bounded role-separated proof wire messages
8. verify opposite proof
9. activate LogicalSession with AuthenticatedConfidentialChannel
10. exchange capability advertisement
11. begin ordinary sequenced control/data work
```

The transport adapter owns bytes and protected stream mechanics. `crosslab-protocol` continues to own hello/proof encoding. `crosslab-core` continues to own transcript/proof/session authentication. Test/application orchestration composes them without importing Quinn types into the domain crates.

A peer that sends ordinary control before successful authentication is rejected. A proof from another connection fails because the exporter binding differs.

## 15. Reconnect and Revocation

M8 reuses M7 exactly:

### Disconnect/reconnect

```text
old QUIC connection closes
  -> old TransportConnection terminal
  -> old LogicalSession Closed
  -> old control/operation state cancelled

new QUIC connection
  -> new exporter binding
  -> fresh nonces
  -> fresh hello/proofs
  -> new SessionId
  -> sequences start from zero
  -> capabilities renegotiated
  -> operations require fresh authorization
```

No old `SessionId`, `AuthorizedOperation`, request sequence, proof, stream, or control record may cross the connection boundary.

### Active peer revocation

Once a newer valid local trust transition changes the authenticated peer to `Revoked`, existing M7 runtime behavior cancels session authority and closes the Quinn adapter. A later Quinn connection may be cryptographically protected at TLS level but Cross-Lab session authentication still fails because policy/trust rejects the device.

## 16. Testing Strategy

M8 is test-driven and keeps tests behavior-focused.

### Adapter tests

Prove:

- loopback Quinn handshake succeeds with explicit test trust;
- both endpoints derive the same 32-byte `quic-tls-exporter-v1` binding;
- a second fresh connection produces a different binding;
- no 0-RTT path is used;
- ordered control records survive fragmentation/coalescing;
- outbound/inbound control queues are bounded;
- oversized control/opening/chunk records are rejected without unbounded allocation;
- multiple unidirectional streams are multiplexed independently;
- per-stream chunk backpressure is bounded;
- finish maps to peer `Finished`;
- cancel maps to QUIC reset/stop semantics;
- connection close maps to transport `Closed` and cancels active stream work;
- shutdown joins owned tasks and endpoint idle completion does not hang.

### End-to-end Cross-Lab scenarios

Add Quinn-backed integration scenarios that reuse production identity, trust, policy, protocol, session, control, and stream code:

1. trusted devices exchange session-auth hello/proofs over QUIC and become `Active` with `AuthenticatedConfidentialChannel`;
2. capability advertisement, control request/response/event flow behaves the same as the memory transport;
3. authorized unidirectional data stream is accepted only with the active session/operation binding;
4. abrupt transport loss closes the logical session and cancels session-scoped authority;
5. reconnect creates a fresh binding and different `SessionId`, resets control sequencing, renegotiates capabilities, and rejects old operation authority;
6. a proof captured from the old connection fails on the new binding;
7. active signed peer revocation cancels work and closes the connection;
8. reconnect after revocation fails before `Active`;
9. cancellation during bootstrap/stream open and shutdown with active streams terminates cleanly.

### Network faults

M8 network fault coverage uses real Quinn loopback connection close, endpoint shutdown/drop, stream reset/stop, and connection loss timing around bootstrap/control/data operations. Packet-loss percentage emulation, NAT simulation, relay failure, and route migration remain M9/later unless a defect requires a narrower regression harness.

## 17. Verification Gates

Every green implementation checkpoint runs focused tests plus the repository baseline. Final M8 verification must include:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If M8 does not change `crates/protocol/**`, `crates/policy/**`, `fuzz/**`, or the fuzz workflow, the existing path-filtered fuzz job is not claimed as a fuzz pass. If protocol wire code changes, the relevant fuzz smoke becomes required.

Final review also verifies that no Quinn/Tokio/rustls type appears in `crosslab-identity`, `crosslab-policy`, `crosslab-protocol`, or `crosslab-core` public domain state.

## 18. Milestone Checkpoints

M8 should progress as small durable checkpoints:

```text
A. approved design + implementation plan + proposed ADR
B. transport-neutral size-bound strengthening
C. Quinn crate + TLS exporter binding + loopback connection
D. bounded control/bootstrap bridge
E. bounded uni-stream bridge
F. Cross-Lab auth/control integration over QUIC
G. reconnect/revocation/fault/shutdown integration
H. final verification + review + CURRENT.md closeout
I. exact verified branch merge to main + canonical main CI
```

`docs/development/CURRENT.md` is updated after meaningful checkpoints and before any interruption-prone handoff.

## 19. Alternatives Considered

### Make `crosslab-core` async/Tokio-aware

Rejected. It would make the stable domain depend on one runtime/transport implementation and weaken the existing deterministic simulator seam.

### Expose `quinn::Connection` to session/runtime code

Rejected. It violates the architecture invariant that transport libraries do not define Cross-Lab domain APIs.

### Create a second Quinn-specific session runtime

Rejected. M5–M7 already implement the security/session/control/stream lifecycle. M8 must prove reuse, not fork it.

### Use certificate fingerprint as channel binding

Rejected for M8. A TLS exporter binds the actual established cryptographic session, while a certificate fingerprint identifies certificate material and does not by itself prove the same live channel.

### Use QUIC 0-RTT for reconnect

Rejected. M7 explicitly requires fresh authentication and fresh operation authority; 0-RTT creates replay/authority complexity with no M8 requirement.

### Add datagrams now

Rejected. No Phase 1 consumer exists.

### Add Iroh/libp2p now

Rejected. M9 explicitly evaluates remote connectivity only after Quinn is measured.

## 20. Architecture/ADR Impact

The overall Master Architecture does not change. M8 implements its already-selected Quinn transport milestone and preserves the approved transport/session boundary.

One deferred security compatibility detail is resolved: the Quinn channel-binding profile. That exact exporter profile is proposed for ADR-0008 because its label/context/output semantics become security-sensitive compatibility data.

No other ADR is required by this design unless implementation evidence forces a material change to the approved transport/session abstraction.

## 21. Approval Gate

Approval of this design authorizes writing the detailed M8 implementation plan and proposed ADR-0008. It does not authorize widening M8 into discovery, relay/NAT, transport migration, platform integration, persistence, or other later milestones.
