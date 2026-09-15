# Foundation Authority and Replay Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make owner/delegated authority currentness locally authoritative, invalidate sessions when their authentication authority is superseded, lock bounded request semantics to ADR-0011, and remove caller-selected trust/policy currentness from high-level stream admission.

**Architecture:** `crosslab-identity` owns an in-memory `OwnerAuthorityState` containing the active owner root plus monotonic delegated-role epoch floors and active delegations. Policy/Core consume that state for authority-bearing operations; sessions snapshot the root and Device Signing authority used at authentication and revalidate them after local authority changes. `message_seq` remains exact-envelope replay/order protection, while `RequestId` remains bounded duplicate/retry history. Stream admission receives authoritative local `TrustRecord` and `PolicyState` objects and derives revisions internally.

**Tech Stack:** Rust 1.98.1 workspace; existing Ed25519/BLAKE3 primitives in `crosslab-crypto`; `crosslab-identity`, `crosslab-policy`, `crosslab-core`, simulator adapters, Quinn transport tests; cargo-audit; cargo-fuzz nightly-2026-09-12.

**Spec:** `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`

## Global Constraints

- ADR-0010 and ADR-0011 are authoritative.
- ADR-0009 remains reserved for M9 remote networking; do not begin M9 Task 4 in this plan.
- Execute from a fresh implementation branch created from verified `main` after the accepted architecture PR is merged.
- Do not change protobuf schemas, canonical transcript bytes, signature formats, identifier formats, Quinn channel-binding bytes, or transport-contract semantics.
- Do not add a database, persistent authority store, CRDT authority merge, global mutable authority registry, daemon, 0-RTT authority, or emergency-root-recovery mechanism.
- `OwnerAuthorityState` belongs to `crosslab-identity` and must not depend on policy, core, protocol, transport, UI, or platform types.
- Root rotation clears active delegated-role objects but retains each role's highest accepted epoch floor.
- Missing current delegated authority fails closed even when a historical epoch floor exists.
- Root or Device Signing replacement invalidates ordinary sessions authenticated under superseded authority; Administrative/Recovery-only replacement does not.
- Request tracking remains bounded; active/in-flight state is never evicted to preserve completed history.
- Peer `RetryClass::Idempotent` never grants duplicate execution authority.
- High-level stream admission derives trust/policy currentness from local state, never caller-selected revision integers.
- Existing golden vectors and externally visible protocol bytes must remain unchanged.

---

## File Structure

- `crates/identity/src/authority_state.rs` — owner-root/delegated-role currentness state.
- `crates/identity/src/{lib.rs,credential.rs}` — export state and route credentials through it.
- `crates/identity/tests/authority_state.rs` — state transition and rollback tests.
- `crates/policy/src/authorization/approval.rs` — current Administrative authority.
- `crates/policy/src/trust/{mod.rs,pairing.rs,transition.rs}` — current Device Signing/root/delegated trust authority.
- `crates/policy/tests/{authorization.rs,trust.rs,trust_transitions.rs}` — stale-authority regressions.
- `crates/core/src/pairing/flow.rs` — state-backed pairing.
- `crates/core/src/session/{state.rs,currentness.rs}` — state-backed authentication and authority revalidation.
- `crates/core/tests/{pairing_flow.rs,session_auth.rs,session_rotation.rs}` plus `session_authority_currentness.rs` — session regressions.
- `crates/core/tests/control_replay.rs` — ADR-0011 characterization.
- `crates/core/src/stream/mod.rs`, `crates/core/tests/stream_admission.rs` — local trust/policy currentness.
- `apps/sim/src/{node.rs,stream.rs}`, `apps/sim/tests/{lifecycle.rs,stream_scenarios.rs}` — runtime cancellation/close behavior.
- `transports/quic/src/session_auth_tests.rs`, `transports/quic/src/session_auth_tests/lifecycle_tests.rs` — fixture migration only.
- `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`, `docs/plans/phase-1/M9-whole-project-security-quality-audit.md`, `docs/development/CURRENT.md` — final evidence.

---

### Task 1: Add identity-owned `OwnerAuthorityState`

**Files:**
- Create: `crates/identity/src/authority_state.rs`
- Create: `crates/identity/tests/authority_state.rs`
- Modify: `crates/identity/src/lib.rs`

**Interfaces:**

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

`OwnerRoot` passed to delegated-slot access returns `WrongIssuerRole`; missing active delegation returns `UnknownIssuer`; equal/lower epoch returns `InvalidAuthorityEpoch`.

- [x] **Step 1: Write the RED monotonicity test**

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

Add separate tests for first observed epoch > 0, wrong owner, delegated `OwnerRoot`, wrong active root/signature, alternate same-epoch key, role independence, and failed-replacement atomicity.

- [x] **Step 2: Run RED**

```bash
cargo test -p crosslab-identity --test authority_state
```

Expected: compile failure because `OwnerAuthorityState` is absent.

- [x] **Step 3: Implement the minimal state**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct DelegatedRoleState {
    accepted_epoch: Option<u64>,
    active: Option<AuthorityDelegation>,
}
```

Use exactly three role slots. Validate owner/role/signature against `self.root` before mutation. When a floor exists, reject `delegation.delegation_epoch() <= floor`; do not compute `floor + 1`.

- [x] **Step 4: Add the RED root-successor test**

```rust
#[test]
fn root_successor_clears_active_role_but_keeps_epoch_floor() {
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

Extend it with epoch 8 under root 1 succeeding, and add failed-successor atomicity.

- [x] **Step 5: Implement root successor acceptance**

Call `successor.verify(&self.root)?` before mutation. On success replace root, clear active delegations, retain all epoch floors.

- [x] **Step 6: Verify**

```bash
cargo fmt --check
cargo test -p crosslab-identity --all-features
cargo clippy -p crosslab-identity --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add crates/identity/src crates/identity/tests/authority_state.rs
git commit -m "feat(identity): add authoritative owner authority state"
```

---

### Task 2: Route device credentials through current authority

**Files:**
- Modify: `crates/identity/src/credential.rs`
- Modify: `crates/identity/tests/credentials.rs`
- Modify: `crates/identity/tests/public_key_issuance.rs`
- Modify: `crates/identity/tests/credential_import.rs`
- Verify unchanged: `crates/identity/tests/golden_vectors.rs`

**Interfaces:**

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

These names are transitional; Task 7 removes the old raw-currentness API and renames these to the ordinary clean names.

- [x] **Step 1: Write RED stale-DSA verification**

Construct state with DSA epoch 0, issue credential C0, accept DSA epoch 1, then assert:

```rust
assert_eq!(
    c0.verify_current(&authority, 0),
    Err(IdentityError::UnknownIssuer)
);
```

Also assert the epoch-0 signing key cannot issue through `issue_current` after epoch 1 is active.

- [x] **Step 2: Run RED**

```bash
cargo test -p crosslab-identity --test credentials superseded_device_signing_authority_cannot_verify_current_credentials
```

Expected: compile failure because `verify_current` is absent.

- [x] **Step 3: Implement current-authority credential methods**

Resolve `DeviceSigning` through `OwnerAuthorityState`. Issuance requires the supplied private key to match the active delegation. Verification requires the credential's issuer key ID to match the active delegation exactly before signature verification. No method accepts a caller delegation floor.

- [x] **Step 4: Preserve credential epoch/wire rules**

Initial credential epoch remains 0; ordinary device rotation remains exact N+1; transcript fields, key derivation, and signatures remain unchanged.

- [x] **Step 5: Verify**

```bash
cargo test -p crosslab-identity --all-features
```

Expected: PASS including golden vectors.

- [x] **Step 6: Commit**

```bash
git add crates/identity/src/credential.rs crates/identity/tests
git commit -m "feat(identity): verify credentials against current authority"
```

---

### Task 3: Migrate policy authority-bearing transitions

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

Owner approval resolves current `Administrative`; pairing and credential rotation resolve current `DeviceSigning`; root revocation uses `authority.root()`.

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

Only `Administrative` and `DeviceSigning` remain valid ordinary delegated-revocation roles.

- [x] **Step 1: Write RED Administrative-currentness tests**

Issue owner approval under Administrative epoch 0, accept epoch 1, and assert old evidence verification plus new issuance with the old key return `ApprovalError::UnknownIssuer`.

- [x] **Step 2: Write RED root/delegated revocation-currentness tests**

Add tests proving old-root revocation cannot apply after a root successor; Administrative/DeviceSigning epoch-0 revocation cannot apply after epoch 1; Recovery remains `WrongIssuerRole`.

- [x] **Step 3: Write RED pairing/credential-rotation currentness tests**

Prove `TrustRecord::accept_successor_credential_current`, `PairingTrustTransition::issue_current`, and `establish_current` reject state tied only to superseded Device Signing authority.

- [x] **Step 4: Implement current-authority policy methods**

Use `OwnerAuthorityState`; no current-authority method accepts `minimum_delegation_epoch`. Keep `PolicyState::evaluate` pure and deterministic.

- [x] **Step 5: Verify**

```bash
cargo fmt --check
cargo test -p crosslab-policy --all-features
cargo clippy -p crosslab-policy --all-targets --all-features -- -D warnings
```

Expected: PASS with golden vectors unchanged.

- [x] **Step 6: Commit**

```bash
git add crates/policy/src crates/policy/tests
git commit -m "feat(policy): enforce current owner authority"
```

---

### Task 4: Migrate pairing/session authentication and authority lifecycle

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

```rust
pub const fn SessionHandshakeSide::new(
    credential: &'a DeviceCredential,
    protocol_ranges: &'a [ProtocolRange],
    features: &'a FeatureSet,
) -> Self;
```

```rust
pub const fn SessionActivation::new(
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

`SessionContext` adds private fields/getters for `root_key_id: KeyId`, `root_epoch: u64`, `device_signing_key_id: KeyId`, `device_signing_epoch: u64`.

`SessionError` adds exactly `OwnerAuthorityChanged` and `DeviceSigningAuthorityChanged`.

```rust
pub fn LogicalSession::revalidate_authority(
    &mut self,
    authority: &OwnerAuthorityState,
) -> Result<(), SessionError>;
```

- [x] **Step 1: Write RED fresh-auth tests**

Create `session_authority_currentness.rs`. First case: credentials are signed under DSA epoch 0, authority advances to DSA epoch 1 before `authenticate`; expect `SessionError::Identity(IdentityError::UnknownIssuer)` and `Closed`. Second case: root successor is accepted and DSA is inactive; expect fresh ordinary auth failure until a strictly newer DSA under the new root is accepted.

- [x] **Step 2: Write RED active-session tests**

Define `AuthoritySessionFixture` in the new test file with concrete keys/IDs/credentials/trust/proofs and methods `active()`, `rotate_device_signing()`, `rotate_root()`, `rotate_administrative()`, `rotate_recovery()`.

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

Add root-successor close plus Administrative/Recovery non-close tests.

- [x] **Step 3: Migrate pairing flow**

Pairing authority-bearing methods take `&OwnerAuthorityState`; remove raw root/delegation/minimum-epoch inputs. Use current-authority credential and policy methods. Do not change pairing transcript/confirmation/credential-acceptance bytes.

- [x] **Step 4: Migrate session authentication**

Resolve root/current DSA from state, verify both credentials with current authority, and snapshot root/DSA IDs and epochs into `SessionContext`. Continue constructing `SessionAuthTranscriptV1` from its existing fields only.

- [x] **Step 5: Implement active authority revalidation**

Compare root snapshot to `authority.root()` first. Root mismatch closes and returns `OwnerAuthorityChanged`. Then resolve current DSA; missing or different DSA closes and returns `DeviceSigningAuthorityChanged`.

- [x] **Step 6: Propagate cancellation in simulator runtimes**

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

Add equivalent behavior to `SimStreamRuntime`; on failure cancel registered operation/stream authority before closing transport. Add lifecycle assertions in the two listed simulator test files.

- [x] **Step 7: Migrate Quinn fixtures**

Construct `OwnerAuthorityState`, accept the fixture DSA, and pass it into `SessionActivation`. Do not place owner authority in Quinn endpoint/TLS/channel-binding types.

- [x] **Step 8: Verify**

```bash
cargo test -p crosslab-core --all-features
cargo test -p crosslab-sim --all-features
cargo test -p crosslab-transport-quic --all-features
```

Expected: PASS.

- [x] **Step 9: Commit**

```bash
git add crates/core apps/sim transports/quic
git commit -m "feat(core): invalidate sessions on authority rotation"
```

---

### Task 5: Lock ADR-0011 bounded replay semantics

**Files:**
- Modify: `crates/core/tests/control_replay.rs`
- Inspect only: `crates/core/src/control/mod.rs`

**Interfaces:** No production API or wire change is planned. Current bounded inbound/completed state plus `ControlSequence` is expected to satisfy ADR-0011.

- [x] **Step 1: Add aged-out-ID characterization**

With `state_capacity = 2`, accept/complete A, B, and C so A leaves completed history. Submit A again at the next valid sequence and assert `Ok(InboundControl::Request(_))` rather than `DuplicateRequest`.

- [x] **Step 2: Prove current policy still applies after eviction**

Repeat after A has aged out but pass `PolicyState::new()`:

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

- [x] **Step 3: Assert exact-envelope replay remains rejected**

After sequence 0 has been accepted, submit another envelope with sequence 0:

```rust
assert_eq!(
    accept_request(
        &mut dispatcher,
        &session,
        request(RequestId::from_bytes([0x55; 16]), RetryClass::NonRetryable),
        0,
        &fixture,
        &policy,
    ),
    Err(ControlDispatchError::Sequence(
        SequenceError::ReplayDetected {
            expected: 1,
            received: 0,
        }
    ))
);
```

Import `DecisionReason` and `SequenceError` in the test.

- [x] **Step 4: Run characterization**

```bash
cargo test -p crosslab-core --test control_replay
```

Expected: PASS with no production change. If it does not, stop this task and use systematic-debugging before touching `control/mod.rs` because the accepted design and current implementation were previously assessed as aligned.

- [x] **Step 5: Commit**

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

```rust
pub fn StreamAdmission::admit_inbound(
    &mut self,
    session: &LogicalSession,
    open: &DataStreamOpen,
    now: u64,
    peer_trust: &TrustRecord,
    policy: &PolicyState,
) -> Result<AdmittedStream, StreamAdmissionError>;
```

Add `PeerTrustMismatch` and `PeerNotTrusted` to `StreamAdmissionError`. Validate trust owner/device against `SessionContext`, require `TrustState::Trusted`, then derive `peer_trust.trust_revision()` and `policy.revision()` internally.

- [x] **Step 1: Rewrite the test fixture and capture RED**

Store `peer_trust: TrustRecord` and `policy: PolicyState` instead of copied revision integers. Remove `admit_with_revisions`; make `admit` call:

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

Expected: compile failure because production still expects two `u64` revisions.

- [x] **Step 2: Implement local-state derivation**

Reject owner/device mismatch with `PeerTrustMismatch`; reject Pending/Revoked with `PeerNotTrusted`; pass only locally derived revisions into `AuthorizedOperation::reserve_stream_use`. Leave low-level `AuthorizedOperation::{validate,reserve_stream_use}` explicit-revision APIs unchanged.

- [x] **Step 3: Add policy/revocation/mismatch tests**

Create an operation under current policy, then increment that same `PolicyState` revision by inserting a second valid unrelated rule; admission must return `OperationError::PolicyRevisionChanged`. Revoke the fixture trust record through a valid `TrustTransition`; admission must return `PeerNotTrusted`. Build a second valid trusted peer record with a different `DeviceId`; admission must return `PeerTrustMismatch`.

- [x] **Step 4: Migrate simulator stream API**

```rust
pub fn SimStreamRuntime::accept_one(
    &mut self,
    now: u64,
    peer_trust: &TrustRecord,
    policy: &PolicyState,
) -> Result<StreamId, SimStreamError>;
```

Update `apps/sim/tests/stream_scenarios.rs`; no high-level simulator caller supplies revision integers.

- [x] **Step 5: Verify**

```bash
cargo test -p crosslab-core --test stream_admission
cargo test -p crosslab-sim --all-features
```

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/core/src/stream crates/core/tests/stream_admission.rs apps/sim/src/stream.rs apps/sim/tests/stream_scenarios.rs
git commit -m "fix(core): derive stream currentness from local state"
```

---

### Task 7: Remove obsolete caller-selected authority APIs

**Files:**
- Modify: `crates/identity/src/credential.rs`
- Modify: `crates/policy/src/authorization/approval.rs`
- Modify: `crates/policy/src/trust/{mod.rs,pairing.rs,transition.rs}`
- Modify remaining fixture call sites across `crates/`, `apps/sim/`, `transports/quic/`.
- Verify golden/vector files remain semantically unchanged.

**Interfaces:** Keep `AuthorityDelegation::verify(root, minimum_epoch)` only as low-level cryptographic/import validation. Keep `DeviceCredential::from_unverified_signed_parts`. Remove ordinary public root/delegation/floor APIs and rename the current-authority methods to the clean ordinary names.

- [x] **Step 1: Remove legacy high-level raw-currentness methods**

Delete or make crate-private old `DeviceCredential::{issue,issue_for_public_key,verify,rotate}` and old approval/pairing/trust-transition methods that let ordinary callers select current authority. Rename `*_current` replacements to ordinary method names after migration.

- [x] **Step 2: Use compiler errors as the call-site checklist**

```bash
cargo check --workspace --all-targets --all-features
```

Expected initially: remaining legacy call sites fail. Migrate every one to `OwnerAuthorityState`; do not add compatibility shims that recreate caller-selected currentness.

- [x] **Step 3: Search for forbidden high-level floors**

```bash
rg "minimum_delegation_epoch|\.verify\([^\n]*root[^\n]*issuer" crates apps transports
```

Expected: `minimum_delegation_epoch` remains only in low-level `AuthorityDelegation::verify` implementation/tests or explicit cryptographic tests that do not establish local currentness.

- [x] **Step 4: Verify golden vectors**

```bash
cargo test -p crosslab-identity --test golden_vectors
cargo test -p crosslab-policy --test golden_vectors
cargo test -p crosslab-protocol --test wire_vectors
```

Expected: byte-for-byte existing vectors PASS unchanged.

- [x] **Step 5: Verify workspace compile/lint**

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates apps/sim transports/quic
git commit -m "refactor(security): remove caller-selected authority currentness"
```

---

### Task 8: Final foundation regression and evidence

**Files:**
- Modify: `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`
- Modify: `docs/plans/phase-1/M9-whole-project-security-quality-audit.md`
- Modify: `docs/development/CURRENT.md`
- Modify: this plan only to check boxes backed by evidence.

**Interfaces:** No new runtime interface; this task proves and records the final foundation state.

- [x] **Step 1: Run complete workspace verification**

```bash
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
```

Expected: PASS. The documented `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning may remain only while isolated to the Iroh experiment and the accepted audit invocation exits successfully.

- [x] **Step 2: Run exact CI fuzz-smoke commands**

```bash
cargo +nightly-2026-09-12 fuzz run control_frame -- -runs=256
cargo +nightly-2026-09-12 fuzz run data_stream_open -- -runs=256
cargo +nightly-2026-09-12 fuzz run identifiers -- -runs=256
cargo +nightly-2026-09-12 fuzz run pairing_bootstrap -- -runs=256
cargo +nightly-2026-09-12 fuzz run session_auth -- -runs=256
```

Expected: all five PASS.

- [x] **Step 3: Reconcile security assessment and audit**

Record only verified facts: ADR-0010 resolved; old root/delegated authority cannot authorize ordinary high-level work after replacement; root/DSA session currentness verified; ADR-0011 bounded semantics verified; stream currentness derives from local trust/policy; no wire/golden changes. Keep deferred risks explicit: production authority persistence/rollback protection, platform key stores, future system-event-family authorization, branch protection, platform CI, and isolated Iroh `paste` warning while present.

- [x] **Step 4: Update `CURRENT.md` with exact evidence**

Record implementation branch, exact head commit, PR number, Rust CI run, Fuzz run, and deferred risks. M9 Task 4 stays blocked until merge and post-merge `main` verification are green.

- [ ] **Step 5: Push and verify exact-head CI/Fuzz**

Require exact-head Rust CI success for audit/fmt/check/Clippy/tests, Fuzz Smoke success, and a PR diff with no unintended protobuf/schema/canonical-vector changes.

- [x] **Step 6: Commit reconciliation**

```bash
git add security docs/development/CURRENT.md docs/plans/phase-1/M9-whole-project-security-quality-audit.md docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md
git commit -m "docs: reconcile final M9 foundation hardening gate"
```

- [ ] **Step 7: Merge and verify `main`**

Merge the implementation PR with its expected exact head SHA, then verify the resulting `main` merge commit with the full Rust CI gate. Only after post-merge `main` is green may `CURRENT.md` identify **M9 Task 4 remote-networking architecture work** as the next implementation task.
