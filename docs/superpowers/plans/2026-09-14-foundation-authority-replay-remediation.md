# Foundation Authority and Replay Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make active owner/delegated authority currentness locally authoritative, invalidate sessions when their authentication authority is superseded, lock bounded request semantics to ADR-0011, and remove caller-selected trust/policy revision currentness from high-level stream admission.

**Architecture:** `crosslab-identity` owns an in-memory `OwnerAuthorityState` containing the active owner root plus monotonic delegated-role epoch floors and active delegations. Policy/Core consume that state for authority-bearing operations; sessions snapshot the root/Device Signing authority used at authentication and revalidate it on authority changes. Existing control `message_seq` remains exact-envelope replay protection, while `RequestId` remains bounded duplicate/retry history. Stream admission receives authoritative local `TrustRecord`/`PolicyState` objects and derives revisions internally.

**Tech Stack:** Rust 1.98.1 workspace, Ed25519/BLAKE3 domain primitives already in `crosslab-crypto`, `crosslab-identity`, `crosslab-policy`, `crosslab-core`, simulator adapters, Quinn transport tests, cargo-audit, cargo-fuzz nightly-2026-09-12.

**Spec:** `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`

## Global Constraints

- ADR-0010 and ADR-0011 are authoritative for this implementation.
- ADR-0009 remains reserved for the M9 remote-networking decision; do not begin M9 Task 4 here.
- Execute this plan from a fresh implementation branch created from the verified `main` commit after the accepted architecture PR is merged.
- No protobuf schema, canonical transcript, signature format, identifier format, Quinn channel-binding, or transport-contract byte change.
- No database, durable persistence subsystem, CRDT authority merge, global mutable authority registry, new daemon, 0-RTT authority, or emergency-root-recovery design.
- `OwnerAuthorityState` stays in `crosslab-identity` and must not depend on policy, core, protocol, transport, UI, or platform types.
- Root rotation clears active delegated-role objects but retains each role's highest accepted epoch floor.
- A missing active delegated role fails closed even when a historical accepted epoch exists.
- Root or Device Signing replacement invalidates ordinary sessions authenticated under superseded authority; Administrative/Recovery-only replacement does not.
- Request tracking stays bounded; active/in-flight request state is never evicted to preserve completed history.
- Peer `RetryClass::Idempotent` never grants duplicate execution authority.
- High-level stream admission derives trust/policy currentness from local state rather than caller-provided revision numbers.
- Preserve existing golden vectors and externally visible protocol bytes.
- Keep comments short and explain only security intent that is not obvious from code.

---

## File Structure

- `crates/identity/src/authority_state.rs` — new identity-owned owner/delegated authority currentness state.
- `crates/identity/src/{lib.rs,credential.rs}` — export state and route high-level credential operations through it.
- `crates/identity/tests/authority_state.rs` — role/root currentness and mutation-atomicity regression tests.
- `crates/policy/src/authorization/approval.rs` — Administrative-authority currentness.
- `crates/policy/src/trust/{mod.rs,pairing.rs,transition.rs}` — Device Signing/root/delegated trust transitions through current authority state.
- `crates/policy/tests/{authorization.rs,trust.rs,trust_transitions.rs}` — stale-authority regressions.
- `crates/core/src/pairing/flow.rs` — pairing flow consumes `OwnerAuthorityState` rather than raw root/delegation/floor.
- `crates/core/src/session/{state.rs,currentness.rs}` — state-backed fresh authentication plus active-session authority snapshot/revalidation.
- `crates/core/tests/{pairing_flow.rs,session_auth.rs,session_rotation.rs}` and new `session_authority_currentness.rs` — current-authority regressions.
- `crates/core/tests/control_replay.rs` — characterization tests for ADR-0011; existing dispatcher code should remain unchanged when those tests pass.
- `crates/core/src/stream/mod.rs`, `crates/core/tests/stream_admission.rs` — derive trust/policy currentness from local authoritative objects.
- `apps/sim/src/{node.rs,stream.rs}`, `apps/sim/tests/{lifecycle.rs,stream_scenarios.rs}` — propagate authority-currentness failure into cancellation/transport close and migrate stream-currentness inputs.
- `transports/quic/src/session_auth_tests.rs`, `transports/quic/src/session_auth_tests/lifecycle_tests.rs` — migrate session fixtures without changing Quinn semantics.
- `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`, `docs/plans/phase-1/M9-whole-project-security-quality-audit.md`, `docs/development/CURRENT.md` — final evidence/reconciliation only after exact-head verification.

---

### Task 1: Add identity-owned `OwnerAuthorityState`

**Files:**
- Create: `crates/identity/src/authority_state.rs`
- Create: `crates/identity/tests/authority_state.rs`
- Modify: `crates/identity/src/lib.rs`

**Interfaces:**
- Consumes: existing `OwnerRootRecord`, `RootSuccessor`, `AuthorityDelegation`, `AuthorityRole`, `IdentityError`.
- Produces:

```rust
pub struct OwnerAuthorityState { /* private fields */ }

impl OwnerAuthorityState {
    pub fn new(root: OwnerRootRecord) -> Self;
    pub fn root(&self) -> &OwnerRootRecord;
    pub fn current_delegation(
        &self,
        role: AuthorityRole,
    ) -> Result<&AuthorityDelegation, IdentityError>;
    pub fn accepted_delegation_epoch(
        &self,
        role: AuthorityRole,
    ) -> Result<Option<u64>, IdentityError>;
    pub fn accept_delegation(
        &mut self,
        delegation: AuthorityDelegation,
    ) -> Result<(), IdentityError>;
    pub fn accept_root_successor(
        &mut self,
        successor: &RootSuccessor,
    ) -> Result<(), IdentityError>;
}
```

- `AuthorityRole::OwnerRoot` is invalid for delegated-slot APIs and returns `IdentityError::WrongIssuerRole`.
- Missing active delegation returns the existing `IdentityError::UnknownIssuer`.
- Equal/lower replacement returns `IdentityError::InvalidAuthorityEpoch`.

- [ ] **Step 1: Write RED state tests**

Create `crates/identity/tests/authority_state.rs` with the strict-monotonicity test:

```rust
#[test]
fn delegation_replacement_is_strictly_monotonic() {
    let owner = OwnerId::from_bytes([0x10; 32]);
    let root_key = SigningKey::from_secret_bytes([0x11; 32]);
    let root = OwnerRootRecord::new(owner, &root_key, 0);
    let key0 = SigningKey::from_secret_bytes([0x12; 32]);
    let key1 = SigningKey::from_secret_bytes([0x13; 32]);
    let d0 = AuthorityDelegation::issue(
        owner,
        AuthorityRole::DeviceSigning,
        &key0,
        4,
        &root_key,
    );
    let d1 = AuthorityDelegation::issue(
        owner,
        AuthorityRole::DeviceSigning,
        &key1,
        5,
        &root_key,
    );

    let mut state = OwnerAuthorityState::new(root);
    state.accept_delegation(d0).unwrap();
    state.accept_delegation(d1).unwrap();

    assert_eq!(
        state.accept_delegation(d0),
        Err(IdentityError::InvalidAuthorityEpoch)
    );
    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::DeviceSigning)
            .unwrap(),
        Some(5)
    );
    assert_eq!(
        state
            .current_delegation(AuthorityRole::DeviceSigning)
            .unwrap()
            .delegated_key_id(),
        d1.delegated_key_id()
    );
}
```

Add separate tests for: first observed epoch greater than zero; wrong owner; delegated `OwnerRoot`; wrong active root/signature; alternate delegation at equal epoch; role independence; failed replacement leaves state unchanged.

- [ ] **Step 2: Run the RED tests**

```bash
cargo test -p crosslab-identity --test authority_state
```

Expected: compile failure because `OwnerAuthorityState` does not exist.

- [ ] **Step 3: Implement the smallest state object**

Use one private focused slot type:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct DelegatedRoleState {
    accepted_epoch: Option<u64>,
    active: Option<AuthorityDelegation>,
}
```

Store exactly three fields of this type in `OwnerAuthorityState`; do not introduce a `HashMap`, trait hierarchy, service object, persistence interface, or global registry.

`accept_delegation` validates owner/role/signature against `self.root` before mutation, then rejects `delegation_epoch() <= accepted_epoch` when a floor exists. Do not compute `floor + 1`, so `u64::MAX` remains overflow-safe.

- [ ] **Step 4: Add root-successor RED coverage**

```rust
#[test]
fn root_successor_clears_active_roles_but_keeps_epoch_floors() {
    let owner = OwnerId::from_bytes([0x20; 32]);
    let root0_key = SigningKey::from_secret_bytes([0x21; 32]);
    let root1_key = SigningKey::from_secret_bytes([0x22; 32]);
    let root0 = OwnerRootRecord::new(owner, &root0_key, 0);
    let dsa7_key = SigningKey::from_secret_bytes([0x23; 32]);
    let dsa7 = AuthorityDelegation::issue(
        owner,
        AuthorityRole::DeviceSigning,
        &dsa7_key,
        7,
        &root0_key,
    );
    let successor = RootSuccessor::issue(&root0, &root0_key, &root1_key).unwrap();

    let mut state = OwnerAuthorityState::new(root0);
    state.accept_delegation(dsa7).unwrap();
    state.accept_root_successor(&successor).unwrap();

    assert_eq!(
        state.current_delegation(AuthorityRole::DeviceSigning),
        Err(IdentityError::UnknownIssuer)
    );
    assert_eq!(
        state
            .accepted_delegation_epoch(AuthorityRole::DeviceSigning)
            .unwrap(),
        Some(7)
    );

    let stale_key = SigningKey::from_secret_bytes([0x24; 32]);
    let stale = AuthorityDelegation::issue(
        owner,
        AuthorityRole::DeviceSigning,
        &stale_key,
        7,
        &root1_key,
    );
    assert_eq!(
        state.accept_delegation(stale),
        Err(IdentityError::InvalidAuthorityEpoch)
    );
}
```

Then add the epoch-8-under-new-root success assertion and a failed-successor atomicity test.

- [ ] **Step 5: Implement root successor acceptance**

Call `successor.verify(&self.root)?` before mutation. On success, replace root, clear only active delegation objects, and retain all accepted epoch floors.

- [ ] **Step 6: Run focused identity verification**

```bash
cargo fmt --check
cargo test -p crosslab-identity --all-features
cargo clippy -p crosslab-identity --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/identity/src crates/identity/tests/authority_state.rs
git commit -m "feat(identity): add authoritative owner authority state"
```

---

### Task 2: Route high-level device credentials through current authority

**Files:**
- Modify: `crates/identity/src/credential.rs`
- Modify: `crates/identity/tests/credentials.rs`
- Modify: `crates/identity/tests/public_key_issuance.rs`
- Modify: `crates/identity/tests/credential_import.rs`
- Verify unchanged: `crates/identity/tests/golden_vectors.rs`

**Interfaces:**
- Consumes: `OwnerAuthorityState::root()` and `current_delegation(AuthorityRole::DeviceSigning)`.
- Produces transitional current-authority methods used until Task 7 removes the raw high-level API:

```rust
pub fn issue_current(
    owner_id: OwnerId,
    device_id: DeviceId,
    device_key: &SigningKey,
    credential_epoch: u64,
    authority: &OwnerAuthorityState,
    issuer_key: &SigningKey,
) -> Result<Self, IdentityError>;

pub fn issue_for_public_key_current(
    owner_id: OwnerId,
    device_id: DeviceId,
    device_public_key: VerifyingKey,
    credential_epoch: u64,
    authority: &OwnerAuthorityState,
    issuer_key: &SigningKey,
) -> Result<Self, IdentityError>;

pub fn verify_current(
    &self,
    authority: &OwnerAuthorityState,
    minimum_credential_epoch: u64,
) -> Result<(), IdentityError>;

pub fn rotate_current(
    &self,
    new_device_key: &SigningKey,
    authority: &OwnerAuthorityState,
    issuer_key: &SigningKey,
) -> Result<Self, IdentityError>;
```

- [ ] **Step 1: Add a RED stale-Device-Signing credential test**

```rust
#[test]
fn superseded_device_signing_authority_cannot_verify_current_credentials() {
    // Arrange authority with DeviceSigning epoch 0 and issue credential C0.
    // Accept a valid DeviceSigning epoch 1 into the same authority state.
    // C0.verify_current(&authority, 0) must return UnknownIssuer.
}
```

Use concrete synthetic keys/IDs following existing credential tests; assert the exact `IdentityError::UnknownIssuer` result.

- [ ] **Step 2: Run the RED test**

```bash
cargo test -p crosslab-identity --test credentials superseded_device_signing_authority_cannot_verify_current_credentials
```

Expected: compile failure because `verify_current` does not exist.

- [ ] **Step 3: Implement current-authority credential helpers**

Resolve the active Device Signing delegation from `OwnerAuthorityState`; never accept a delegation floor from the caller. `issue_current`/`issue_for_public_key_current` require `issuer_key.verifying_key()` to match the active delegation. `verify_current` requires `issuer_device_signing_key_id` to match the active delegation exactly, then verifies signature/credential epoch using existing primitives.

- [ ] **Step 4: Preserve credential epoch and wire rules**

Keep initial credential epoch `0`, normal device credential rotation exact `N+1`, transcript fields, key derivation, and signature bytes unchanged.

- [ ] **Step 5: Run identity tests and vectors**

```bash
cargo test -p crosslab-identity --all-features
```

Expected: PASS including existing golden vectors.

- [ ] **Step 6: Commit**

```bash
git add crates/identity/src/credential.rs crates/identity/tests
git commit -m "feat(identity): verify credentials against current authority"
```

---

### Task 3: Migrate policy authority-bearing state transitions

**Files:**
- Modify: `crates/policy/src/authorization/approval.rs`
- Modify: `crates/policy/src/trust/mod.rs`
- Modify: `crates/policy/src/trust/pairing.rs`
- Modify: `crates/policy/src/trust/transition.rs`
- Modify: `crates/policy/tests/authorization.rs`
- Modify: `crates/policy/tests/trust.rs`
- Modify: `crates/policy/tests/trust_transitions.rs`
- Verify unchanged: `crates/policy/tests/golden_vectors.rs`

**Interfaces:**
- Owner approval resolves `AuthorityRole::Administrative` from `OwnerAuthorityState`.
- Pairing and credential rotation resolve current `DeviceSigning` authority.
- Root revocation uses `authority.root()`.
- Delegated revocation takes an explicit requested role but resolves the exact current delegation from state:

```rust
pub fn issue_delegated_revocation_current(
    record: &TrustRecord,
    transition_id: TransitionId,
    authority: &OwnerAuthorityState,
    issuer_role: AuthorityRole,
    issuer_key: &SigningKey,
) -> Result<Self, TrustTransitionError>;

pub fn apply_delegated_current(
    &self,
    record: &mut TrustRecord,
    authority: &OwnerAuthorityState,
) -> Result<(), TrustTransitionError>;
```

Only `Administrative` and `DeviceSigning` remain allowed for ordinary delegated revocation; `OwnerRoot` and `Recovery` stay rejected.

- [ ] **Step 1: Write RED Administrative-currentness tests**

Create Administrative delegation epoch 0, issue approval evidence through it, accept epoch 1, then assert:

```rust
assert_eq!(
    old_evidence.verify_current(&authority),
    Err(ApprovalError::UnknownIssuer)
);
```

Also assert the epoch-0 signing key cannot create new current approval evidence once epoch 1 is active.

- [ ] **Step 2: Write RED root/delegated revocation-currentness tests**

Add separate tests proving:

- a revocation signed by root 0 cannot apply through `apply_root_current` after state accepts root 1;
- a revocation signed by Administrative/DeviceSigning epoch 0 cannot apply after epoch 1 is current;
- Recovery delegation remains `WrongIssuerRole` for ordinary revocation.

- [ ] **Step 3: Write RED credential-rotation/pairing currentness tests**

Prove `TrustRecord::accept_successor_credential_current`, `PairingTrustTransition::issue_current`, and `establish_current` reject credentials/transitions that depend only on a superseded Device Signing delegation.

- [ ] **Step 4: Implement current-authority policy methods**

Route verification through `OwnerAuthorityState`; no new current-authority method accepts `minimum_delegation_epoch`. Keep `PolicyState::evaluate` pure and deterministic; it must not perform key verification or authority mutation.

- [ ] **Step 5: Run policy verification**

```bash
cargo fmt --check
cargo test -p crosslab-policy --all-features
cargo clippy -p crosslab-policy --all-targets --all-features -- -D warnings
```

Expected: PASS, golden vectors unchanged.

- [ ] **Step 6: Commit**

```bash
git add crates/policy/src crates/policy/tests
git commit -m "feat(policy): enforce current owner authority"
```

---

### Task 4: Migrate pairing/session authentication and authority-triggered lifecycle

**Files:**
- Modify: `crates/core/src/pairing/flow.rs`
- Modify: `crates/core/src/session/state.rs`
- Modify: `crates/core/src/session/currentness.rs`
- Modify: `crates/core/tests/pairing_flow.rs`
- Modify: `crates/core/tests/session_auth.rs`
- Modify: `crates/core/tests/session_rotation.rs`
- Create: `crates/core/tests/session_authority_currentness.rs`
- Modify: `apps/sim/src/node.rs`
- Modify: `apps/sim/src/stream.rs`
- Modify: `apps/sim/tests/lifecycle.rs`
- Modify: `apps/sim/tests/stream_scenarios.rs`
- Modify: `transports/quic/src/session_auth_tests.rs`
- Modify: `transports/quic/src/session_auth_tests/lifecycle_tests.rs`

**Interfaces:**

`SessionHandshakeSide` no longer carries raw issuer delegation:

```rust
pub const fn new(
    credential: &'a DeviceCredential,
    protocol_ranges: &'a [ProtocolRange],
    features: &'a FeatureSet,
) -> Self;
```

`SessionActivation` receives authoritative owner state instead of raw root:

```rust
pub const fn new(
    authority: &'a OwnerAuthorityState,
    initiator: SessionHandshakeSide<'a>,
    responder: SessionHandshakeSide<'a>,
    local_role: SessionAuthRole,
    peer_trust: &'a TrustRecord,
    initiator_nonce: [u8; 32],
    responder_nonce: [u8; 32],
    channel_binding: &'a ChannelBinding,
    transport_security_class: TransportSecurityClass,
    initiator_proof: &'a SessionAuthProof,
    responder_proof: &'a SessionAuthProof,
) -> Self;
```

`SessionContext` stores local authentication-basis metadata with getters:

```rust
root_key_id: KeyId,
root_epoch: u64,
device_signing_key_id: KeyId,
device_signing_epoch: u64,
```

`SessionError` adds exactly:

```rust
OwnerAuthorityChanged,
DeviceSigningAuthorityChanged,
```

`LogicalSession` adds:

```rust
pub fn revalidate_authority(
    &mut self,
    authority: &OwnerAuthorityState,
) -> Result<(), SessionError>;
```

- [ ] **Step 1: Write RED fresh-session stale-authority tests**

In `session_authority_currentness.rs`:

1. authenticate fixtures are prepared with Device Signing epoch 0 credentials;
2. authority accepts Device Signing epoch 1 before `session.authenticate(...)`;
3. authentication must fail before `Active` with `SessionError::Identity(IdentityError::UnknownIssuer)`.

Add a second test: accept root successor, leaving Device Signing inactive; fresh ordinary authentication must fail until a strictly newer Device Signing delegation is accepted under the new root.

- [ ] **Step 2: Write RED active-session authority-currentness tests**

```rust
#[test]
fn device_signing_rotation_closes_active_session() {
    let mut fixture = AuthoritySessionFixture::active();
    fixture.rotate_device_signing();

    assert_eq!(
        fixture.session.revalidate_authority(&fixture.authority),
        Err(SessionError::DeviceSigningAuthorityChanged)
    );
    assert_eq!(fixture.session.state(), SessionState::Closed);
}
```

Define `AuthoritySessionFixture` inside this new test file with concrete synthetic keys, owner/device IDs, credentials, trust record, proofs, and helper methods `active()`, `rotate_device_signing()`, `rotate_root()`, `rotate_administrative()`, and `rotate_recovery()`. Add root-successor test plus Administrative/Recovery negative tests.

- [ ] **Step 3: Migrate pairing flow**

Change inviter/joiner authority-bearing methods to take `&OwnerAuthorityState`; remove raw root/delegation/minimum-epoch arguments. Use `DeviceCredential::*_current` and policy `*_current` methods. Pairing transcript, confirmation, and credential-acceptance bytes remain unchanged.

- [ ] **Step 4: Migrate fresh session authentication**

Resolve root and current Device Signing delegation from `OwnerAuthorityState`, call `verify_current`, and snapshot root/DSA key IDs + epochs into `SessionContext`. Continue building `SessionAuthTranscriptV1` from the same existing inputs so its bytes do not change.

- [ ] **Step 5: Implement `revalidate_authority`**

Compare the context snapshot to `authority.root()` first, then to `authority.current_delegation(AuthorityRole::DeviceSigning)`. A root mismatch returns `OwnerAuthorityChanged`; a missing or changed Device Signing delegation returns `DeviceSigningAuthorityChanged`. On either error, transition through the existing close path and finish `Closed`.

- [ ] **Step 6: Propagate lifecycle cancellation in simulator runtimes**

Add to `SimNode`:

```rust
pub fn revalidate_authority(
    &mut self,
    authority: &OwnerAuthorityState,
) -> Result<(), NodeError> {
    if let Err(error) = self.session.revalidate_authority(authority) {
        self.dispatcher.cancel_session_state();
        self.transport.close();
        return Err(NodeError::Session(error));
    }
    Ok(())
}
```

Add the same entry point to `SimStreamRuntime`; on error call its existing session-authority cancellation helper before closing transport, so registered operations and active streams are cancelled.

Add lifecycle assertions in `apps/sim/tests/lifecycle.rs` and `apps/sim/tests/stream_scenarios.rs` proving authority-currentness loss clears pending control/operation/stream authority and closes transport.

- [ ] **Step 7: Migrate Quinn/session fixtures**

Construct one `OwnerAuthorityState` per fixture, accept the Device Signing delegation into it, and pass it to `SessionActivation`. Quinn endpoint/TLS/channel-binding types remain unchanged and never become authority state.

- [ ] **Step 8: Run focused core/simulator/Quinn tests**

```bash
cargo test -p crosslab-core --all-features
cargo test -p crosslab-sim --all-features
cargo test -p crosslab-transport-quic --all-features
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/core apps/sim transports/quic
git commit -m "feat(core): invalidate sessions on authority rotation"
```

---

### Task 5: Lock ADR-0011 bounded replay semantics with characterization tests

**Files:**
- Modify: `crates/core/tests/control_replay.rs`
- Inspect only: `crates/core/src/control/mod.rs`

**Interfaces:**
- No production API/wire change is planned.
- Existing bounded `inbound_requests`, `completed_inbound`, `completed_order`, and directional `ControlSequence` are the expected implementation.

- [ ] **Step 1: Add aged-out-ID characterization**

Use capacity 2. Accept and complete request A, then B, then C so A falls out of completed history. Submit A again using the next valid `message_seq` and the same allowed policy.

```rust
assert!(matches!(
    accept_request(
        &mut dispatcher,
        &session,
        request(a, RetryClass::NonRetryable),
        3,
        &fixture,
        &policy,
    ),
    Ok(InboundControl::Request(_))
));
```

- [ ] **Step 2: Prove current authorization still applies after eviction**

Build a fresh dispatcher/session sequence scenario where A is aged out, then submit A with the next valid sequence but pass `PolicyState::new()`.

```rust
assert_eq!(
    accept_request(
        &mut dispatcher,
        &session,
        request(a, RetryClass::NonRetryable),
        next_sequence,
        &fixture,
        &PolicyState::new(),
    ),
    Err(ControlDispatchError::AuthorizationDenied(
        DecisionReason::NoMatchingRule
    ))
);
```

Import `DecisionReason` into the test.

- [ ] **Step 3: Preserve exact-envelope replay protection**

After consuming sequence 0, submit another request envelope with sequence 0 and assert a `ControlDispatchError::Sequence(SequenceError::Replay { .. })` or the exact existing replay variant exposed by `ControlSequence`. Read `crates/protocol/src/control/sequence.rs` before writing the assertion and use its actual variant name; do not change that enum for this task.

- [ ] **Step 4: Run the characterization suite**

```bash
cargo test -p crosslab-core --test control_replay
```

Expected: PASS against the current dispatcher implementation. A failure means the implementation/spec relationship was misunderstood; stop this task and use the systematic-debugging workflow before changing `control/mod.rs`.

- [ ] **Step 5: Commit**

```bash
git add crates/core/tests/control_replay.rs
git commit -m "test(core): lock bounded request replay semantics"
```

---

### Task 6: Derive stream currentness from local trust/policy state

**Files:**
- Modify: `crates/core/src/stream/mod.rs`
- Modify: `crates/core/tests/stream_admission.rs`
- Modify: `apps/sim/src/stream.rs`
- Modify: `apps/sim/tests/stream_scenarios.rs`

**Interfaces:**

Replace caller-selected numeric currentness with:

```rust
pub fn admit_inbound(
    &mut self,
    session: &LogicalSession,
    open: &DataStreamOpen,
    now: u64,
    peer_trust: &TrustRecord,
    policy: &PolicyState,
) -> Result<AdmittedStream, StreamAdmissionError>;
```

Add exactly:

```rust
PeerTrustMismatch,
PeerNotTrusted,
```

to `StreamAdmissionError`.

Before `AuthorizedOperation::reserve_stream_use`, validate that `peer_trust.owner_id()` and `device_id()` match authenticated session context and `peer_trust.state() == TrustState::Trusted`. Then pass `peer_trust.trust_revision()` and `policy.revision()` internally.

- [ ] **Step 1: Rewrite fixture to retain authoritative objects and capture RED**

Change `Fixture` in `stream_admission.rs` to store:

```rust
peer_trust: TrustRecord,
policy: PolicyState,
```

Remove copied `trust_revision` and `policy_revision` fields and remove `admit_with_revisions`. Make `admit` call:

```rust
admission.admit_inbound(
    &self.session,
    open,
    15,
    &self.peer_trust,
    &self.policy,
)
```

Run:

```bash
cargo test -p crosslab-core --test stream_admission
```

Expected: compile failure because the production signature still accepts two `u64` revisions.

- [ ] **Step 2: Implement local-state derivation**

Import `PolicyState`, `TrustRecord`, and `TrustState`. Reject mismatched owner/device as `PeerTrustMismatch` and Pending/Revoked as `PeerNotTrusted`. Pass only locally derived revisions into `reserve_stream_use`.

Keep `AuthorizedOperation::validate` and `reserve_stream_use` explicit-revision parameters as lower-level policy primitives for focused policy tests.

- [ ] **Step 3: Add revocation/current-policy tests**

Add a test that creates an operation under the fixture's current policy, advances the policy by inserting a second unrelated rule, then attempts admission using the same `PolicyState`; expect `OperationError::PolicyRevisionChanged`.

Add a revocation test using a valid `TrustTransition` to mutate the stored `TrustRecord` to `Revoked`, then attempt admission; expect `StreamAdmissionError::PeerNotTrusted` before stream reservation.

Add a wrong-peer record test built through a second valid pairing fixture and expect `PeerTrustMismatch`.

- [ ] **Step 4: Migrate `SimStreamRuntime::accept_one`**

Use exactly:

```rust
pub fn accept_one(
    &mut self,
    now: u64,
    peer_trust: &TrustRecord,
    policy: &PolicyState,
) -> Result<StreamId, SimStreamError>;
```

Update `apps/sim/tests/stream_scenarios.rs`; no simulator caller may declare current revision numbers.

- [ ] **Step 5: Run focused stream tests**

```bash
cargo test -p crosslab-core --test stream_admission
cargo test -p crosslab-sim --all-features
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/stream crates/core/tests/stream_admission.rs apps/sim/src/stream.rs apps/sim/tests/stream_scenarios.rs
git commit -m "fix(core): derive stream currentness from local state"
```

---

### Task 7: Remove obsolete caller-selected authority APIs and verify compatibility

**Files:**
- Modify: `crates/identity/src/credential.rs`
- Modify: `crates/policy/src/authorization/approval.rs`
- Modify: `crates/policy/src/trust/{mod.rs,pairing.rs,transition.rs}`
- Modify remaining fixture call sites across `crates/`, `apps/sim/`, and `transports/quic/`.
- Verify golden/vector files remain semantically unchanged.

**Interfaces:**
- Remove ordinary public methods that accept raw root + delegation + caller-selected delegation floor where a current-authority method now exists.
- Keep `AuthorityDelegation::verify(root, minimum_epoch)` as low-level cryptographic/import validation only.
- Keep raw import constructors such as `DeviceCredential::from_unverified_signed_parts` because they do not establish currentness.

- [ ] **Step 1: Remove legacy high-level raw-currentness methods**

Delete or make crate-private the old `DeviceCredential::{issue,issue_for_public_key,verify,rotate}` and the old approval/pairing/trust-transition methods that let ordinary callers select the current root/delegation/floor directly. Rename the `*_current` methods to the clean ordinary names only after all call sites are migrated, so the final API does not retain unnecessary suffix noise.

- [ ] **Step 2: Use compiler errors as the migration checklist**

```bash
cargo check --workspace --all-targets --all-features
```

Expected initially: compile errors at remaining legacy call sites. Migrate every production/test fixture to `OwnerAuthorityState`; do not add compatibility shims that reintroduce caller-selected currentness.

- [ ] **Step 3: Verify no production caller-selected delegation floors remain**

Run:

```bash
rg "minimum_delegation_epoch|\.verify\([^\n]*root[^\n]*issuer" crates apps transports
```

Expected: `minimum_delegation_epoch` remains only in the low-level `AuthorityDelegation::verify` implementation/tests or similarly explicit cryptographic tests that do not claim local currentness. No high-level production credential/policy/core call path may accept it.

- [ ] **Step 4: Re-run all cryptographic/protocol golden vectors**

```bash
cargo test -p crosslab-identity --test golden_vectors
cargo test -p crosslab-policy --test golden_vectors
cargo test -p crosslab-protocol --test wire_vectors
```

Expected: byte-for-byte existing vectors PASS unchanged.

- [ ] **Step 5: Run workspace compile after API cleanup**

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates apps/sim transports/quic
git commit -m "refactor(security): remove caller-selected authority currentness"
```

---

### Task 8: Final foundation regression, security evidence, and handoff

**Files:**
- Modify: `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`
- Modify: `docs/plans/phase-1/M9-whole-project-security-quality-audit.md`
- Modify: `docs/development/CURRENT.md`
- Modify: `docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md` only to check completed boxes backed by evidence.

**Interfaces:**
- No new runtime interface. This task proves the accepted foundation implementation and records exact evidence.

- [ ] **Step 1: Run complete workspace verification**

```bash
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
```

Expected: all commands PASS. The documented `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning may remain only while it is isolated to the Iroh experiment and the repository's accepted audit invocation exits successfully.

- [ ] **Step 2: Run the exact bounded fuzz smoke commands used by CI**

```bash
cargo +nightly-2026-09-12 fuzz run control_frame -- -runs=256
cargo +nightly-2026-09-12 fuzz run data_stream_open -- -runs=256
cargo +nightly-2026-09-12 fuzz run identifiers -- -runs=256
cargo +nightly-2026-09-12 fuzz run pairing_bootstrap -- -runs=256
cargo +nightly-2026-09-12 fuzz run session_auth -- -runs=256
```

Expected: all five bounded fuzz runs PASS.

- [ ] **Step 3: Reconcile the security assessment**

Update `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md` with verified statements only:

- ADR-0010 authority currentness resolved;
- old root/delegated authority no longer accepted by ordinary high-level paths;
- root/Device Signing active-session currentness behavior verified;
- ADR-0011 bounded request semantics aligned in spec/tests;
- stream currentness derives from local trust/policy state;
- no wire/golden change;
- deferred risks: production authority persistence/rollback protection, platform key stores, system-event family authorization when introduced, branch protection, platform CI, and isolated Iroh `paste` warning while still present.

Update the whole-project audit finding/disposition sections to point to the accepted ADRs and exact tests rather than leaving the findings open.

- [ ] **Step 4: Update `CURRENT.md` with exact implementation evidence**

Record the implementation branch, exact head commit, PR number, Rust CI run, Fuzz run, and remaining deferred risks. Keep M9 Task 4 blocked until merge and post-merge `main` verification are green.

- [ ] **Step 5: Push and verify exact-head CI/Fuzz**

Require on the exact implementation head:

- Rust CI success for dependency audit, fmt, check, Clippy, and complete workspace tests;
- Fuzz Smoke success for affected parser/security surfaces;
- PR diff contains no unintended protobuf/schema/canonical-vector change.

- [ ] **Step 6: Commit the final reconciliation**

```bash
git add security docs/development/CURRENT.md docs/plans/phase-1/M9-whole-project-security-quality-audit.md docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md
git commit -m "docs: reconcile final M9 foundation hardening gate"
```

- [ ] **Step 7: Merge only after exact-head evidence is green**

Merge the implementation PR with its expected exact head SHA, then verify the resulting `main` merge commit with the full Rust CI gate. Only after post-merge `main` is green may `CURRENT.md` identify **M9 Task 4 remote-networking architecture work** as the next implementation task.
