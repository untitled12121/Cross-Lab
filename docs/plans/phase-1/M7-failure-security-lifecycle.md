# M7 Failure and Security Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove deterministic disconnect/reconnect, stale-authority rejection, active revocation, reconnect-after-revocation denial, bounded failure behavior, cancellation, and clean shutdown without introducing real networking or resumable session authority.

**Architecture:** Keep signed trust mutation in `crosslab-policy`, logical-session transitions in `crosslab-core`, and deterministic transport/runtime reaction in `crosslab-sim`. Reconnect always creates a new transport and a new `LogicalSession`; revocation reacts only to an already-verified matching `TrustRecord::Revoked`, cancels session-scoped authority, and closes locally.

**Tech Stack:** Rust workspace, existing Ed25519/BLAKE3 identity/session primitives, protobuf protocol domain types, synchronous bounded in-memory transport (`Arc<Mutex<_>>` + bounded `VecDeque`), GitHub Actions CI/fuzz smoke.

**Spec:** `docs/plans/phase-1/M7-failure-security-lifecycle-design.md`

## Global Constraints

- Baseline is exact verified `main` commit `d419f0fd410f0d2da69cc9bf2e4c20600033b7fb`.
- Preserve Master Architecture, `SESSION-TRANSPORT.md`, `PAIRING-TRUST-REVOCATION.md`, and `CORE-SIMULATOR.md` semantics; no architecture change or ADR is expected.
- Reconnect is close + fresh transport/binding/nonces/proofs + fresh `SessionId`; never resurrect an old `LogicalSession`.
- Old request/sequence/operation authority never transfers across sessions.
- Only an already-verified local revoked trust record may trigger ordinary peer-revocation reaction.
- No Quinn/Iroh/libp2p, sockets/TLS, async runtime, reconnect timers/backoff, persistence, UI/platform, privileged service, session ticket, 0-RTT, or recovery reauthorization work enters M7.
- Reuse existing bounded queues/errors; do not add unbounded state, busy polling, detached work, or payload logging.
- Every production slice is test-first and independently committed after focused + workspace verification.

---

### Task 1: Core logical-session terminal lifecycle

**Files:**
- Modify: `crates/core/src/session/state.rs`
- Modify: `crates/core/tests/session.rs`

**Interfaces:**
- Consumes: existing `LogicalSession`, `SessionContext`, `TrustRecord`, `TrustState`, `SessionState`.
- Produces:
  - `LogicalSession::transport_lost(&mut self) -> Result<(), SessionError>`
  - `LogicalSession::apply_peer_revocation(&mut self, peer_trust: &TrustRecord) -> Result<(), SessionError>`
  - `SessionError::PeerNotRevoked`
  - `SessionError::PeerTrustRevisionNotAdvanced`

- [ ] **Step 1: Extend the session fixture so tests can create a real signed revocation**

Keep the root signing key in the existing test fixture and import `TrustTransition`. Use the real M3 transition path rather than mutating trust directly:

```rust
let transition = TrustTransition::issue_root_revocation(
    &fixture.responder_trust,
    TransitionId::from_bytes([0xa0; 32]),
    &fixture.root,
    &fixture.root_key,
)
.unwrap();
let mut revoked = fixture.responder_trust;
transition.apply_root(&mut revoked, &fixture.root).unwrap();
```

- [ ] **Step 2: Write failing transport-loss lifecycle tests**

Add tests proving an active session closes immediately on transport loss and repeated notification is idempotent:

```rust
session.transport_lost().unwrap();
assert_eq!(session.state(), SessionState::Closed);
assert!(session.context().is_some());
session.transport_lost().unwrap();
assert_eq!(session.state(), SessionState::Closed);
```

Also prove `Created` rejects `transport_lost()` with `SessionError::InvalidState`.

- [ ] **Step 3: Write failing peer-revocation validation tests**

Cover:

```rust
session.apply_peer_revocation(&revoked).unwrap();
assert_eq!(session.state(), SessionState::Revoked);
session.finish_close().unwrap();
assert_eq!(session.state(), SessionState::Closed);
```

And rejection for:

```rust
assert_eq!(
    active.apply_peer_revocation(&fixture.responder_trust),
    Err(SessionError::PeerNotRevoked)
);
```

Create matching negative records for wrong owner/device, credential epoch mismatch, and unchanged/non-advanced trust revision; assert the session remains `Active` after each rejection.

- [ ] **Step 4: Run focused tests and verify RED**

Run:

```bash
cargo test -p crosslab-core --test session --all-features
```

Expected: compile/test failure because `transport_lost`, `apply_peer_revocation`, and the two new typed errors do not exist.

- [ ] **Step 5: Implement the minimal core lifecycle APIs**

Add display strings for the new errors and implement:

```rust
pub fn transport_lost(&mut self) -> Result<(), SessionError> {
    match self.state {
        SessionState::Active | SessionState::Closing | SessionState::Revoked => {
            self.state = SessionState::Closed;
            Ok(())
        }
        SessionState::Closed => Ok(()),
        SessionState::Created | SessionState::Authenticating => Err(SessionError::InvalidState),
    }
}
```

For revocation, validate the active context before mutating state:

```rust
if peer_trust.state() != TrustState::Revoked {
    return Err(SessionError::PeerNotRevoked);
}
if peer_trust.owner_id() != context.owner_id()
    || peer_trust.device_id() != context.peer_device_id()
{
    return Err(SessionError::PeerTrustMismatch);
}
if peer_trust.accepted_credential_epoch() != context.peer_credential_epoch() {
    return Err(SessionError::PeerCredentialEpochMismatch);
}
if peer_trust.trust_revision() <= context.peer_trust_revision() {
    return Err(SessionError::PeerTrustRevisionNotAdvanced);
}
self.state = SessionState::Revoked;
```

Do not verify trust-transition signatures here; policy already did that before the record reached this API.

- [ ] **Step 6: Verify Task 1 GREEN**

Run:

```bash
cargo fmt --check
cargo test -p crosslab-core --test session --all-features
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all pass.

- [ ] **Step 7: Commit Task 1**

```bash
git add crates/core/src/session/state.rs crates/core/tests/session.rs
git commit -m "feat(session): add terminal loss and revocation lifecycle"
```

---

### Task 2: Control runtime cleanup on disconnect, revocation, and shutdown

**Files:**
- Modify: `crates/core/src/control/mod.rs`
- Modify: `apps/sim/src/node.rs`
- Modify: `apps/sim/tests/session_scenarios.rs`

**Interfaces:**
- Consumes: Task 1 session lifecycle APIs.
- Produces:
  - `ControlDispatcher::cancel_session_state(&mut self)`
  - `SimNode::apply_peer_revocation(&mut self, peer_trust: &TrustRecord) -> Result<(), NodeError>`
  - `SimNode::shutdown(&mut self)`

- [ ] **Step 1: Write a failing dispatcher cleanup assertion**

Extend control/runtime tests so terminal cleanup removes pending state:

```rust
node_a.send_request(request(request_id, RetryClass::NonRetryable, b"pending"))?;
assert_eq!(node_a.pending_request_count(), 1);
pair.faults().disconnect_now();
assert!(matches!(
    node_a.send_event(system_event(EventId::from_bytes([0xa1; 16]))),
    Err(NodeError::Send(ControlSendError::Closed(_)))
));
assert_eq!(node_a.session().state(), SessionState::Closed);
assert_eq!(node_a.pending_request_count(), 0);
```

Use explicit matches rather than erasing the ownership-preserving transport error.

- [ ] **Step 2: Add receive-side unexpected disconnect RED coverage**

Create an active node, call `disconnect_now()`, then:

```rust
assert_eq!(node_b.session().state(), SessionState::Active);
assert_eq!(
    node_b.receive_one(),
    Err(NodeError::Receive(ControlReceiveError::Closed))
);
assert_eq!(node_b.session().state(), SessionState::Closed);
```

- [ ] **Step 3: Add control-side active revocation RED coverage**

Use a valid signed transition to update the local peer trust record, then:

```rust
node_b.apply_peer_revocation(&revoked_peer).unwrap();
assert_eq!(node_b.session().state(), SessionState::Closed);
assert!(endpoint_b.is_closed());
assert_eq!(node_b.pending_request_count(), 0);
```

Also pass a revoked record for the wrong device and assert `NodeError::Session(SessionError::PeerTrustMismatch)` while the unrelated session remains active.

- [ ] **Step 4: Run the focused simulator test and verify RED**

Run:

```bash
cargo test -p crosslab-sim --test session_scenarios --all-features
```

Expected: failure because the new cleanup/revocation/shutdown APIs are missing and unexpected transport closure does not yet close active sessions.

- [ ] **Step 5: Implement explicit dispatcher terminal cleanup**

Add:

```rust
pub fn cancel_session_state(&mut self) {
    self.pending_outgoing.clear();
    self.inbound_requests.clear();
    self.seen_nonretryable.clear();
}
```

Do not reset state for reuse; a reconnect constructs a fresh dispatcher.

- [ ] **Step 6: Refine `SimNode` fail-closed behavior**

Introduce small private helpers rather than duplicating state transitions:

```rust
fn terminate_transport_loss(&mut self) {
    self.dispatcher.cancel_session_state();
    let _ = self.session.transport_lost();
}

fn finish_terminal_close(&mut self) {
    self.dispatcher.cancel_session_state();
    match self.session.state() {
        SessionState::Active => {
            let _ = self.session.begin_close();
            let _ = self.session.finish_close();
        }
        SessionState::Closing | SessionState::Revoked => {
            let _ = self.session.finish_close();
        }
        SessionState::Created | SessionState::Authenticating | SessionState::Closed => {}
    }
    self.transport.close();
}
```

On `ControlReceiveError::Closed`, terminate an active session. On `ControlSendError::Closed(frame)`, terminate before returning the original error. Leave `Full(frame)` nonterminal and do not commit the outbound dispatcher state until send succeeds.

Implement revocation in the required order:

```rust
self.session.apply_peer_revocation(peer_trust)?;
self.dispatcher.cancel_session_state();
self.transport.close();
self.session.finish_close()?;
```

Implement idempotent `shutdown()` using `finish_terminal_close()`.

- [ ] **Step 7: Verify Task 2 GREEN**

Run:

```bash
cargo fmt --check
cargo test -p crosslab-sim --test session_scenarios --all-features
cargo test -p crosslab-core --all-features
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all pass.

- [ ] **Step 8: Commit Task 2**

```bash
git add crates/core/src/control/mod.rs apps/sim/src/node.rs apps/sim/tests/session_scenarios.rs
git commit -m "feat(sim): terminate control authority on lifecycle failure"
```

---

### Task 3: Stream runtime owns and terminates its logical session

**Files:**
- Modify: `apps/sim/src/stream.rs`
- Modify: `apps/sim/tests/stream_scenarios.rs`

**Interfaces:**
- Consumes: Task 1 session lifecycle APIs and existing `StreamAdmission::cancel_all()`.
- Produces:
  - `SimStreamRuntime::new(session: LogicalSession, transport: &dyn TransportConnection, capacity: NonZeroUsize)`
  - `SimStreamRuntime::session(&self) -> &LogicalSession`
  - `SimStreamRuntime::apply_peer_revocation(&mut self, peer_trust: &TrustRecord) -> Result<(), SimStreamError>`
  - terminal `shutdown()` that also closes the owned logical session.

- [ ] **Step 1: Convert existing stream scenario fixtures to move sessions into runtimes and verify RED**

Change construction from borrowed sessions:

```rust
let sender = SimStreamRuntime::new(
    fixture.sender_session,
    sender_endpoint,
    NonZeroUsize::new(4).unwrap(),
)?;
```

and similarly for receiver. Make sender mutable where `open_uni` must react to a closed transport.

Expected RED: current constructor expects `&LogicalSession` and no session getter exists.

- [ ] **Step 2: Add stream disconnect lifecycle tests**

With an active receiver runtime:

```rust
pair.faults().disconnect_now();
assert!(matches!(
    receiver.accept_one(15, trust_revision, policy_revision),
    Err(SimStreamError::Accept(StreamAcceptError::Closed))
));
assert_eq!(receiver.session().state(), SessionState::Closed);
```

For an active admitted stream, disconnect then receive:

```rust
assert!(matches!(
    receiver.try_receive_chunk(stream_id),
    Err(SimStreamError::Receive(StreamReceiveError::Cancelled))
));
assert_eq!(receiver.session().state(), SessionState::Closed);
```

- [ ] **Step 3: Add stream revocation and shutdown RED coverage**

After admitting one stream and registering an operation:

```rust
receiver.apply_peer_revocation(&revoked_peer).unwrap();
assert_eq!(receiver.session().state(), SessionState::Closed);
assert!(receiver_endpoint.is_closed());
assert!(matches!(
    receiver.try_receive_chunk(stream_id),
    Err(SimStreamError::StreamNotFound)
));
```

For shutdown:

```rust
receiver.shutdown();
receiver.shutdown();
assert_eq!(receiver.session().state(), SessionState::Closed);
assert!(receiver_endpoint.is_closed());
```

- [ ] **Step 4: Run focused tests and verify RED**

Run:

```bash
cargo test -p crosslab-sim --test stream_scenarios --all-features
```

Expected: compile/test failure on the new ownership/lifecycle contract.

- [ ] **Step 5: Implement session ownership with small terminal helpers**

Change the struct field to:

```rust
session: LogicalSession,
```

and update active checks to borrow `&self.session`.

Add:

```rust
pub const fn session(&self) -> &LogicalSession {
    &self.session
}
```

Use a helper for transport loss:

```rust
fn transport_lost(&mut self) {
    self.cancel_active_streams();
    self.admission.cancel_all();
    let _ = self.session.transport_lost();
}
```

Only treat stream cancellation as connection loss when `self.transport.is_closed()`; stream-local cancellation on an otherwise open transport stays stream-local.

Implement revocation in this order:

```rust
self.session.apply_peer_revocation(peer_trust)?;
self.cancel_active_streams();
self.admission.cancel_all();
self.transport.close();
self.session.finish_close()?;
```

Change `open_uni` to `&mut self` so `StreamOpenError::Closed` can close the owned session; keep `Full`/`TooLarge` nonterminal.

- [ ] **Step 6: Verify Task 3 GREEN**

Run:

```bash
cargo fmt --check
cargo test -p crosslab-sim --test stream_scenarios --all-features
cargo test -p crosslab-sim --test memory_stream_transport --all-features
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all pass.

- [ ] **Step 7: Commit Task 3**

```bash
git add apps/sim/src/stream.rs apps/sim/tests/stream_scenarios.rs
git commit -m "feat(sim): terminate stream authority with session lifecycle"
```

---

### Task 4: S-008 fresh reconnect and stale-authority rejection

**Files:**
- Create: `apps/sim/tests/lifecycle.rs`

**Interfaces:**
- Consumes: Task 1–3 lifecycle APIs, existing session-auth helpers/domain types, `MemoryTransportPair`, `ControlDispatcher`, `StreamAdmission`, `AuthorizedOperation`.
- Produces: deterministic S-008 coverage only; no new production API.

- [ ] **Step 1: Build one deterministic lifecycle fixture**

The fixture owns owner/root/delegation/device keys/credentials/trust and helpers:

```rust
fn sessions(
    &self,
    pair: &MemoryTransportPair,
    initiator_nonce: [u8; 32],
    responder_nonce: [u8; 32],
) -> (LogicalSession, LogicalSession)
```

The helper always builds fresh proofs from the supplied pair binding and nonces. Never reuse proof objects across reconnect.

Add helpers for the existing `files.transfer/send` capability and an allowed `AuthorizedOperation` bound to a supplied session.

- [ ] **Step 2: Write S-008 disconnect/reconnect test**

Use pair A/binding A/nonces A:

```rust
let old_session_id = old_a.context().unwrap().session_id();
```

Exchange capabilities, create pending control state, disconnect pair A, and assert the old node observes `Closed` with no pending requests.

Use pair B with a different binding and fresh nonces:

```rust
assert_ne!(
    new_a.context().unwrap().session_id(),
    old_session_id
);
assert_eq!(new_a.context().unwrap().next_send_sequence(), 0);
assert_eq!(new_a.context().unwrap().next_receive_sequence(), 0);
assert!(new_a.context().unwrap().negotiated_capabilities().is_empty());
```

Then exchange capabilities again and assert the negotiated set is restored only after the fresh exchange.

- [ ] **Step 3: Write old control-state replay rejection test**

Encode an envelope using `old_session_id` and inject it into pair B. The new node must return:

```rust
Err(NodeError::Dispatch(ControlDispatchError::InvalidSession))
```

and fail closed. Use a separate fresh session instance for later assertions so one deliberate fatal replay does not contaminate the rest of S-008.

- [ ] **Step 4: Write old operation rejection against the new session**

Create an `AuthorizedOperation` under old session A. Register it in a new `StreamAdmission`, but build `DataStreamOpen` with the new session ID and old `OperationId`:

```rust
assert_eq!(
    admission.admit_inbound(
        &new_session,
        &open,
        15,
        trust_revision,
        policy_revision,
    ),
    Err(StreamAdmissionError::Operation(OperationError::BindingMismatch))
);
```

This proves possession of an old operation ID cannot authorize a new logical session.

- [ ] **Step 5: Run lifecycle tests**

Run:

```bash
cargo test -p crosslab-sim --test lifecycle --all-features
```

Expected: PASS if Tasks 1–3 correctly enforce fresh-state semantics.

- [ ] **Step 6: Run the workspace baseline and commit Task 4**

Run:

```bash
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Then:

```bash
git add apps/sim/tests/lifecycle.rs
git commit -m "test(sim): prove fresh reconnect lifecycle"
```

---

### Task 5: S-009 active revocation and reconnect-after-revocation denial

**Files:**
- Modify: `apps/sim/tests/lifecycle.rs`

**Interfaces:**
- Consumes: existing signed `TrustTransition`, Task 2 control revocation reaction, Task 3 stream revocation reaction.
- Produces: S-009, N-040, and N-041 end-to-end coverage.

- [ ] **Step 1: Add a helper that accepts a real signed peer revocation**

Use the owner root path:

```rust
fn revoke_peer(
    record: &mut TrustRecord,
    root: &OwnerRootRecord,
    root_key: &SigningKey,
    transition_byte: u8,
) {
    let transition = TrustTransition::issue_root_revocation(
        record,
        TransitionId::from_bytes([transition_byte; 32]),
        root,
        root_key,
    )
    .unwrap();
    transition.apply_root(record, root).unwrap();
}
```

- [ ] **Step 2: Prove active control authority terminates locally**

Start a trusted active session and one pending control request. Apply the signed transition to B's local trust record, then:

```rust
node_b.apply_peer_revocation(&revoked_a).unwrap();
assert_eq!(node_b.session().state(), SessionState::Closed);
assert_eq!(node_b.pending_request_count(), 0);
assert!(endpoint_b.is_closed());
```

Verify subsequent ordinary send/receive work is rejected by the closed session/transport.

- [ ] **Step 3: Prove active stream authority terminates locally**

Create a separate fresh active session with one admitted `SingleStream` operation and an active stream. Apply the same trust transition pattern and call stream-runtime revocation:

```rust
receiver.apply_peer_revocation(&revoked_sender).unwrap();
assert_eq!(receiver.session().state(), SessionState::Closed);
assert!(receiver_endpoint.is_closed());
assert!(send.try_send_chunk(vec![9]).is_err());
```

Assert receiver runtime no longer exposes the admitted stream.

- [ ] **Step 4: Prove N-041 reconnect/auth after revocation is denied**

Build a completely new pair/binding/nonces/proofs using the same still-cryptographically-valid device credential but pass the revoked local `TrustRecord` into `SessionActivation`:

```rust
let mut reconnect = LogicalSession::new();
assert_eq!(
    reconnect.authenticate(activation_with_revoked_trust),
    Err(SessionError::PeerNotTrusted)
);
assert_eq!(reconnect.state(), SessionState::Closed);
```

Do not rotate the credential or introduce recovery semantics.

- [ ] **Step 5: Prove unrelated/malformed revocation state cannot kill a session**

Use a revoked record for another `DeviceId` and a still-trusted unchanged record. Both must return typed session errors and leave the intended session active.

- [ ] **Step 6: Verify Task 5 and commit**

Run:

```bash
cargo fmt --check
cargo test -p crosslab-sim --test lifecycle --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Then:

```bash
git add apps/sim/tests/lifecycle.rs
git commit -m "test(sim): prove active revocation lifecycle"
```

---

### Task 6: Failure/resource/shutdown closeout without duplicate coverage

**Files:**
- Modify: `apps/sim/tests/session_scenarios.rs`
- Modify: `apps/sim/tests/stream_scenarios.rs`
- Modify only if a missing transport invariant is discovered: `apps/sim/tests/memory_transport.rs`
- Modify only if a missing stream-transport invariant is discovered: `apps/sim/tests/memory_stream_transport.rs`

**Interfaces:**
- Consumes: existing typed backpressure/cancellation errors and Task 2–3 terminal cleanup.
- Produces: explicit N-042..N-046 lifecycle/resource acceptance mapping; no new production abstraction unless a failing test proves one is necessary.

- [ ] **Step 1: Inventory existing N-020..N-046 tests before adding anything**

Map exact existing test names for replay, malformed/oversized frame, sequence replay/gap, duplicate request, control saturation, stream saturation, request cancellation, stream setup cancellation, and shutdown. Record the mapping as comments in the implementation checkpoint/PR body, not in production code.

- [ ] **Step 2: Add the missing control backpressure transaction assertion**

Use control capacity 1. Fill the transport queue, then attempt another `SimNode` send and assert:

```rust
let sequence_before = node_a.next_send_sequence();
let pending_before = node_a.pending_request_count();
assert!(matches!(
    node_a.send_request(second_request),
    Err(NodeError::Send(ControlSendError::Full(_)))
));
assert_eq!(node_a.next_send_sequence(), sequence_before);
assert_eq!(node_a.pending_request_count(), pending_before);
assert_eq!(node_a.session().state(), SessionState::Active);
```

Drain capacity and retry successfully. This proves backpressure does not spend sequence/request state.

- [ ] **Step 3: Strengthen fatal-input cleanup assertions**

For existing malformed-frame / invalid-session / sequence-failure tests, create pending request state before the fatal input where practical and assert afterward:

```rust
assert_eq!(node.session().state(), SessionState::Closed);
assert_eq!(node.pending_request_count(), 0);
assert!(endpoint.is_closed());
```

Do not duplicate parser boundary tests already owned by M4 fuzz/unit coverage.

- [ ] **Step 4: Strengthen deterministic shutdown tests**

Control runtime:

```rust
node.shutdown();
node.shutdown();
assert_eq!(node.session().state(), SessionState::Closed);
assert_eq!(node.pending_request_count(), 0);
assert!(endpoint.is_closed());
```

Stream runtime:

```rust
runtime.shutdown();
runtime.shutdown();
assert_eq!(runtime.session().state(), SessionState::Closed);
assert!(endpoint.is_closed());
```

Keep existing active-stream sender cancellation assertion.

- [ ] **Step 5: Run full test/fuzz baseline**

Run locally:

```bash
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Push the exact code head and require GitHub CI plus existing Fuzz Smoke to pass. Do not add a new fuzz target unless production work introduced a new untrusted parser; this plan introduces none.

- [ ] **Step 6: Commit Task 6**

```bash
git add apps/sim/tests/session_scenarios.rs apps/sim/tests/stream_scenarios.rs apps/sim/tests/memory_transport.rs apps/sim/tests/memory_stream_transport.rs
git commit -m "test(sim): close M7 failure lifecycle matrix"
```

If the two transport test files were unchanged, omit them from `git add`; do not create meaningless diffs.

---

### Task 7: Final scope review, durable handoff, and integration

**Files:**
- Modify: `docs/development/CURRENT.md`
- Modify if implementation decisions materially refine wording only: `docs/plans/phase-1/M7-failure-security-lifecycle-design.md`
- Modify if actual task boundaries changed during verified execution: `docs/plans/phase-1/M7-failure-security-lifecycle.md`

**Interfaces:**
- Consumes: exact verified heads/runs from Tasks 1–6.
- Produces: final M7 PR evidence, canonical `main` integration record, exact M8 next task.

- [ ] **Step 1: Self-review the entire branch diff against M7 scope**

Confirm the changed files contain only:

```text
core logical-session/control lifecycle
sim control/stream lifecycle reaction
S-008/S-009 + missing failure/resource/shutdown tests
M7 docs/CURRENT
```

Reject accidental Quinn/Iroh/libp2p, async runtime, socket/TLS, persistence, UI/platform, recovery, reconnect timer/backoff, or M8 work.

- [ ] **Step 2: Update `CURRENT.md` with exact evidence**

Record:

- branch/base exact SHA;
- each valid RED commit + CI run;
- each GREEN/refinement head + CI/fuzz run;
- final code head and full locked/rustfmt/check/Clippy/test/fuzz evidence;
- S-008/S-009/N-040..N-046 coverage mapping;
- explicit statement that reconnect creates new session authority and revocation uses verified local trust state;
- exact next task: M8 Quinn transport planning/ADR work from verified canonical `main`.

- [ ] **Step 3: Verify the documentation-inclusive PR head**

Require on the exact head:

```text
cargo metadata --locked
rustfmt
workspace check
Clippy -D warnings
all workspace tests
existing Fuzz Smoke
```

Do not mark the PR ready while any exact-head run is pending/failing.

- [ ] **Step 4: Perform final review and merge only the verified head**

Confirm no unresolved review threads and compare `main...head`. Mark ready and merge with expected-head protection using preserved history.

- [ ] **Step 5: Verify canonical `main` after merge**

Require push-triggered `main` CI success. Confirm the merge tree matches the verified PR tree when possible.

- [ ] **Step 6: Write and verify final integration record on `main`**

Update `CURRENT.md` on `main` with merge SHA, post-merge CI, final documentation head, and M8 handoff. Verify the documentation-only `main` head before calling M7 complete.
