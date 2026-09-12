# M7 Failure and Security Lifecycle — Design

**Status:** Approved direction; written design checkpoint  
**Milestone:** Phase 1 / M7  
**Baseline:** verified canonical `main` after M6 final integration (`d419f0fd410f0d2da69cc9bf2e4c20600033b7fb`)

## 1. Goal

M7 proves that failure cannot preserve stale Cross-Lab authority.

The milestone covers deterministic disconnect/reconnect, replay rejection across fresh sessions, active peer revocation, reconnect-after-revocation denial, malformed/resource failure behavior, cancellation, and clean shutdown using the existing in-memory simulator.

The governing lifecycle is:

```text
active logical session
        |
        +-- graceful close ---------> Closed
        |
        +-- transport loss ---------> Closed
        |
        +-- accepted peer revocation -> Revoked -> Closed

reconnect
  = new transport
  + new binding
  + fresh nonces/proofs
  + new SessionId
  + capability renegotiation
  + fresh authorization
```

M7 does not implement session tickets, 0-RTT authorization, operation transfer, cross-session replay tolerance, transport migration, real networking, reconnect timers/backoff, persistence, UI, or M8 Quinn integration.

## 2. Source Contracts

M7 implements already-approved architecture rather than changing it.

The Master Architecture requires:

- logical sessions, not concrete transports, own reconnect/revocation semantics;
- Phase 1 reconnect is clean disconnect plus fresh authentication/state restoration;
- core owns disconnect/reconnect handling, revocation reaction, bounded queues, shutdown, and cancellation;
- M7 proves reconnect, replay rejection, active revocation, reconnect-after-revocation denial, malformed input behavior, and resource limits;
- disconnect/reconnect must not bypass fresh authentication;
- revocation must terminate/invalidate active authority and prevent future trusted sessions.

`SESSION-TRANSPORT.md` further freezes these semantics:

- `Active -> Closed` on transport loss/fatal protocol error;
- `Active -> Revoked -> Closed` on accepted peer revocation;
- reconnect creates a completely fresh session with new binding/nonces/proofs/`SessionId`;
- old control sequence/request/operation state is not transferable;
- active revocation rejects new work, invalidates peer-bound operations, stops new stream admission, cancels active work, and closes without peer acknowledgement.

`PAIRING-TRUST-REVOCATION.md` keeps ordinary revocation terminal for a `DeviceId`; a higher credential epoch cannot silently clear `TrustState::Revoked`.

`CORE-SIMULATOR.md` requires S-008, S-009, N-040, N-041, queue saturation, cancellation during operation/stream setup, malformed input handling, and clean shutdown.

No ADR is required because this design preserves those contracts.

## 3. Existing Baseline and Reuse

M7 reuses the existing production-domain work instead of duplicating it:

- M3 `TrustRecord` + signed `TrustTransition` remain the only ordinary revocation authority path;
- M5 `LogicalSession` owns authenticated identity, `SessionId`, negotiated protocol/features/capabilities, trust revision snapshot, and explicit states;
- M5 `ControlDispatcher` owns bounded request/replay/sequence state;
- M5 `SimNode` composes one logical session, control dispatcher, policy, local capabilities, and one transport endpoint;
- M6 `StreamAdmission` owns session-bound operation and stream admission state;
- M6 `SimStreamRuntime` composes stream admission with transport streams;
- M6 `MemoryTransportPair` already provides bounded control/stream/chunk queues, explicit close propagation, `disconnect_now`, directional close, and cancellation.

Research reuse is semantic only:

- uploaded Quinn demonstrates that connection close is terminal for connection-scoped work and that explicit application close and connection loss are distinct transport outcomes;
- uploaded Syncthing demonstrates that reconnect scheduling/backoff belongs above individual connection objects;
- uploaded RustDesk demonstrates that transient link heuristics/reconnect timing are product/network orchestration concerns rather than security-session authority.

M7 therefore models deterministic lifecycle transitions only. Retry timing/backoff remains outside the core session contract.

## 4. Ownership Boundaries

### `crosslab-policy`

Policy continues to own trust records, signed trust transition verification, operation authority, and operation terminal state.

M7 does not add an alternate revocation mutation API. A simulator first applies a valid `TrustTransition` to its local `TrustRecord`; session/runtime code only reacts to that already-accepted local trust state.

### `crosslab-core::session`

Core owns the security-sensitive logical-session transitions.

M7 adds focused APIs for:

- unexpected transport loss/fatal local connection termination;
- applying an already-accepted peer-revocation record to the authenticated session.

Core validates that revocation belongs to the authenticated owner/device, is actually `Revoked`, uses the same accepted credential epoch, and advances the trust revision beyond the snapshot used at authentication.

Core does not verify the revocation signature itself; `TrustTransition::apply_*` already owns that responsibility in policy.

### `crosslab-core::control`

The control dispatcher remains request/sequence state only. M7 adds explicit session-state cancellation/cleanup so pending request authority can be deterministically discarded before a runtime closes.

It does not reconnect itself and does not own transport handles.

### `crosslab-core::stream`

`StreamAdmission` remains operation/stream authority state. Existing `cancel_all()` is the canonical terminal cleanup path for registered operations and admitted streams.

M7 does not move transport work or trust verification into stream admission.

### `crosslab-sim`

Simulator runtimes translate deterministic transport and trust events into the production core lifecycle APIs.

`SimNode` owns control-session termination behavior.

`SimStreamRuntime` is changed from borrowing `LogicalSession` to owning it. This lets stream cleanup and session lifecycle transition happen through one owner without `Arc<Mutex<LogicalSession>>`, `RefCell`, detached tasks, or a new god runtime.

The control and stream simulator features remain separate; M7 integration scenarios may use each feature-focused runtime to prove its relevant lifecycle semantics rather than introducing a monolithic session daemon before M8.

## 5. Logical Session Lifecycle API

The current state machine remains:

```text
Created -> Authenticating -> Active -> Closing -> Closed
                             |
                             +-----------> Closed   transport loss/fatal error
                             |
                             +-> Revoked -> Closed  accepted peer revocation
```

M7 adds two explicit operations.

### Transport loss

Conceptually:

```rust
pub fn transport_lost(&mut self) -> Result<(), SessionError>
```

Semantics:

- `Active`, `Closing`, or `Revoked` become `Closed`;
- `Closed` is idempotent success;
- `Created`/`Authenticating` are rejected because active simulator runtimes are created only after authentication; authentication failures already close inside `authenticate`;
- the authenticated context may remain readable for diagnostics/audit correlation, but state gates prevent reuse as active authority.

### Accepted peer revocation

Conceptually:

```rust
pub fn apply_peer_revocation(
    &mut self,
    peer_trust: &TrustRecord,
) -> Result<(), SessionError>
```

Validation before transition:

- session is `Active`;
- `peer_trust.state() == TrustState::Revoked`;
- owner matches `SessionContext.owner_id`;
- device matches `SessionContext.peer_device_id`;
- accepted credential epoch matches the credential epoch authenticated into the session;
- trust revision is strictly greater than the session snapshot.

Success transitions `Active -> Revoked`. Runtime cleanup then cancels session-scoped work, closes transport, and calls the existing terminal close path `Revoked -> Closed`.

This preserves a testable `Revoked` transition without requiring remote acknowledgement.

## 6. Control Runtime Failure Semantics

`SimNode` must never remain `Active` after observing a closed underlying connection.

M7 behavior:

- `try_receive_control() == Closed` while active -> cancel dispatcher session state, `transport_lost()`, return typed receive error;
- `try_send_control() == Closed(frame)` -> cancel dispatcher session state, `transport_lost()`, return the original ownership-preserving send error;
- malformed/fatal protocol/sequence failure -> existing fail-closed behavior plus dispatcher cleanup;
- graceful `SessionClose` -> existing `Closing -> Closed` path plus cleanup;
- accepted peer revocation -> session `Revoked`, dispatcher cleanup, transport close, session `Closed`;
- explicit runtime shutdown -> cleanup + graceful/terminal session close + transport close; repeated shutdown is harmless.

`ControlDispatcher` receives one narrow terminal cleanup method that clears pending outgoing requests, inbound request state, and nonretryable replay bookkeeping for the dead session. It does not create a new sequence epoch; a reconnect constructs a new dispatcher from a new `SessionContext` whose sequence state starts at zero.

## 7. Stream Runtime Failure Semantics

`SimStreamRuntime` owns its `LogicalSession` in M7.

Conceptually:

```rust
pub fn new(
    session: LogicalSession,
    transport: &dyn TransportConnection,
    capacity: NonZeroUsize,
) -> Result<Self, SimStreamError>

pub fn session(&self) -> &LogicalSession

pub fn apply_peer_revocation(
    &mut self,
    peer_trust: &TrustRecord,
) -> Result<(), SimStreamError>
```

Terminal cleanup order is:

1. stop accepting/opening new work by transitioning session out of `Active`;
2. cancel all inbound transport streams;
3. `StreamAdmission::cancel_all()` to cancel registered session-scoped operations;
4. close the transport;
5. finish session closure.

For transport loss observed through stream open/accept/receive:

- closed/cancelled transport state closes the logical session;
- local stream-only cancellation remains stream-local if the parent transport is still open;
- no operation budget or admitted stream survives `shutdown`/revocation as usable authority.

M7 does not add background stream tasks; all cleanup remains deterministic and synchronous.

## 8. Reconnect Semantics — S-008

S-008 uses two distinct `MemoryTransportPair` instances.

Old session:

1. authenticate on pair A using binding A and nonce pair A;
2. exchange capabilities;
3. create representative session-scoped control/request/operation state;
4. record old `SessionId`;
5. inject transport disconnect;
6. observe old logical session become `Closed` and pending/session-scoped runtime state terminate.

Fresh session:

1. construct pair B with a different channel binding;
2. use fresh initiator/responder nonces;
3. build fresh proofs and authenticate again against current trusted records;
4. assert new `SessionId != old SessionId`;
5. assert control sequence state begins at zero;
6. assert negotiated capabilities start empty and must be exchanged again;
7. prove an old control envelope/request state cannot be accepted by the new session;
8. prove an old `AuthorizedOperation` cannot authorize an action/stream bound to the new session.

No reconnect method resurrects an existing `LogicalSession`. Reconnect means constructing a new logical session from fresh authentication material.

## 9. Active Revocation Semantics — S-009 / N-040 / N-041

The simulator constructs a valid owner-authorized `TrustTransition` using the existing M3 API and applies it to the peer's local `TrustRecord`.

After local trust becomes `Revoked`:

- control runtime rejects new ordinary work;
- pending control request state is cancelled;
- stream runtime cancels active admitted streams and registered operations;
- session takes `Active -> Revoked -> Closed`;
- transport closes without waiting for peer acknowledgement;
- active sender/receiver stream handles become terminal through transport cancellation;
- a fresh authentication attempt using the same cryptographically valid device credential and revoked trust record fails before `Active`;
- a higher credential epoch is not introduced as a bypass in M7.

The session reaction accepts only a revocation record matching the authenticated peer. A revoked record for another owner/device, an unchanged revision, or a still-`Trusted` record is rejected and must not terminate unrelated authority.

## 10. Replay, Malformed Input, Resource Limits, Cancellation

M7 closes gaps rather than duplicating existing M4–M6 coverage.

Existing tests already cover substantial parts of:

- replayed session proof under fresh nonce/binding;
- wrong channel binding;
- malformed/oversized protocol frames;
- control sequence replay/gap;
- duplicate nonretryable request IDs;
- bounded control dispatcher state;
- bounded memory control queues;
- bounded stream-open/chunk/runtime state;
- cancellation during control request and stream setup;
- stream/runtime shutdown.

Before adding new tests, implementation reviews these existing cases and maps them to N-020..N-028 and N-040..N-046.

M7 adds only missing lifecycle assertions, especially:

- unexpected transport disconnect closes active control/stream sessions;
- ownership-preserving backpressure remains recoverable without state growth;
- explicit shutdown leaves no pending requests, active admitted streams, or open memory transport;
- fatal malformed/sequence failures clean session-owned runtime state, not only transport state.

No new fuzz target is needed unless M7 introduces a new untrusted parser. The expected design introduces none.

## 11. Error Layering

Typed errors remain separated by owner:

- invalid trust transition signature/authority -> `TrustTransitionError`;
- rejected session revocation record/state transition -> `SessionError`;
- request/sequence capacity -> `ControlDispatchError`;
- transport queue/close -> existing control/stream transport errors;
- stream admission/operation invalidation -> existing `StreamAdmissionError` / `OperationError`;
- simulator composition wraps these without replacing them with strings.

Sensitive control/stream payload bytes remain redacted from `Debug` where existing transport error wrappers already preserve ownership.

A transport close must not be silently converted into success when a caller needs to know its operation was not sent/received.

## 12. File Structure

Expected focused changes:

```text
crates/core/src/session/state.rs
crates/core/src/control/mod.rs
crates/core/tests/session.rs
crates/core/tests/control.rs or existing control-facing tests

apps/sim/src/node.rs
apps/sim/src/stream.rs
apps/sim/tests/control.rs
apps/sim/tests/stream_scenarios.rs
apps/sim/tests/lifecycle.rs          # new S-008/S-009 focused scenarios

docs/plans/phase-1/M7-failure-security-lifecycle-design.md
docs/plans/phase-1/M7-failure-security-lifecycle.md
docs/development/CURRENT.md
```

Exact test-file reuse is decided from the existing suite during implementation; no unrelated restructuring is planned.

## 13. Security Invariants

M7 must preserve all of these invariants:

1. A dead transport never leaves its logical session usable as `Active` authority.
2. Reconnect always creates a new authenticated `LogicalSession` and new `SessionId`.
3. Old control sequence/request/replay state is never transferred to the new session.
4. Old `AuthorizedOperation` authority is session-bound and cannot authorize the new session.
5. Only an already-verified local revocation record can trigger the ordinary peer-revocation reaction.
6. Revocation for one owner/device cannot terminate another authenticated peer session.
7. Accepted peer revocation cancels local session-scoped authority before/while transport closes; no peer acknowledgement is required.
8. `TrustState::Revoked` prevents future ordinary authentication even with a cryptographically valid credential.
9. Queue saturation remains bounded and ownership-preserving.
10. Shutdown/cancellation is deterministic, idempotent where appropriate, and leaves no active simulator authority.

## 14. Out of Scope

M7 explicitly excludes:

- Quinn/rustls or any real socket transport;
- Iroh/libp2p integration;
- reconnect backoff/timers/network discovery;
- seamless transport migration;
- session resumption tickets/0-RTT;
- operation or request transfer across sessions;
- persistent trust/session stores;
- UI/platform/mobile code;
- privileged services;
- audit persistence;
- recovery reauthorization or un-revocation.

Those require later milestone-specific design.

## 15. Acceptance

M7 is complete when the deterministic simulator proves:

- S-008 clean disconnect + fresh authenticated reconnect + distinct `SessionId` + fresh capabilities/sequence/authorization;
- S-009 accepted active revocation terminates current ordinary authority and closes locally;
- N-040 active revocation cancels affected operations/session;
- N-041 reconnect/authentication after revocation is denied;
- existing replay/malformed/oversized/resource tests remain green and any missing lifecycle assertions are added;
- queue saturation and cancellation do not create unbounded or dangling state;
- explicit shutdown closes sessions/transports and cancels session-scoped authority;
- full locked metadata, rustfmt, workspace check, Clippy `-D warnings`, workspace tests, and existing fuzz smoke pass on the exact final branch head;
- verified work is merged to `main`, followed by canonical post-merge verification and a durable `CURRENT.md` handoff to M8.
