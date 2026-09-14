# Foundation Authority and Replay Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make active owner/delegated authority currentness locally authoritative, invalidate sessions when their authentication authority is superseded, lock bounded request semantics to ADR-0011, and remove caller-selected trust/policy revision currentness from high-level stream admission.

**Architecture:** `crosslab-identity` owns an in-memory `OwnerAuthorityState` containing the active owner root plus monotonic delegated-role epoch floors and active delegations. Policy/Core consume that state for authority-bearing operations; sessions snapshot the root/Device Signing authority used at authentication and revalidate it on authority changes. Existing control `message_seq` remains exact-envelope replay protection, while `RequestId` remains bounded duplicate/retry history. Stream admission receives authoritative local `TrustRecord`/`PolicyState` objects and derives revisions internally.

**Tech Stack:** Rust 1.98.1 workspace, Ed25519/BLAKE3 domain primitives already in `crosslab-crypto`, `crosslab-identity`, `crosslab-policy`, `crosslab-core`, simulator adapters, Quinn transport tests, cargo-audit, cargo-fuzz nightly-2026-09-12.

**Spec:** `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`

## Global Constraints

- ADR-0010 and ADR-0011 are authoritative for this implementation.
- ADR-0009 remains reserved for the M9 remote-networking decision; do not begin M9 Task 4 here.
- No protobuf schema, canonical transcript, signature format, identifier format, Quinn channel-binding, or transport contract byte change.
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
- `crates/identity/src/{lib.rs,error.rs,credential.rs}` — export state and route high-level credential operations through it.
- `crates/identity/tests/authority_state.rs` — role/root currentness and mutation-atomicity regression tests.
- `crates/policy/src/authorization/approval.rs` — Administrative-authority currentness.
- `crates/policy/src/trust/{mod.rs,pairing.rs,transition.rs}` — Device Signing/root/delegated trust transitions through current authority state.
- `crates/policy/tests/{authorization.rs,trust.rs,trust_transitions.rs}` — stale authority regressions.
- `crates/core/src/pairing/flow.rs` — pairing flow consumes `OwnerAuthorityState` rather than raw root/delegation/floor.
- `crates/core/src/session/{state.rs,currentness.rs}` — state-backed fresh authentication plus active-session authority snapshot/revalidation.
- `crates/core/tests/{pairing_flow.rs,session_auth.rs,session_rotation.rs}` and new `session_authority_currentness.rs` — current authority regressions.
- `crates/core/src/control/mod.rs`, `crates/core/tests/control_replay.rs` — lock ADR-0011 behavior; production changes only if characterization exposes a mismatch.
- `crates/core/src/stream/mod.rs`, `crates/core/tests/stream_admission.rs` — derive trust/policy currentness from local authoritative objects.
- `apps/sim/src/{node.rs,stream.rs}` plus simulator lifecycle tests — propagate authority-currentness failure into cancellation/transport close.
- Existing Quinn/session fixtures under `transports/quic/src/` — migrate construction to current authority APIs without changing transport semantics.
- `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`, `docs/plans/phase-1/M9-whole-project-security-quality-audit.md`, `docs/development/CURRENT.md` — final evidence/reconciliation only after exact-head verification.

---

### Task 1: Add identity-owned `OwnerAuthorityState`

**Files:**
- Create: `crates/identity/src/authority_state.rs`
- Create: `crates/identity/tests/authority_state.rs`
- Modify: `crates/identity/src/lib.rs`
- Modify only if a focused new variant is truly needed: `crates/identity/src/error.rs`

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
- Missing active delegation returns `IdentityError::UnknownIssuer` unless implementation evidence justifies one narrowly scoped new error.
- Equal/lower replacement returns `IdentityError::InvalidAuthorityEpoch`.

- [ ] **Step 1: Write RED state tests**

Create `crates/identity/tests/authority_state.rs` with focused tests equivalent to:

```rust
#[test]
fn delegation_replacement_is_strictly_monotonic() {
    let owner = OwnerId::from_bytes([0x10; 32]);
    let root_key = SigningKey::from_secret_bytes([0x11; 32]);
    let root = OwnerRootRecord::new(owner, &root_key, 0);
    let key0 = SigningKey::from_secret_bytes([0x12; 32]);
    let key1 = SigningKey::from_secret_bytes([0x13; 32]);
    let d0 = AuthorityDelegation::issue(owner, AuthorityRole::DeviceSigning, &key0, 4, &root_key);
    let d1 = AuthorityDelegation::issue(owner, AuthorityRole::DeviceSigning, &key1, 5, &root_key);

    let mut state = OwnerAuthorityState::new(root);
    state.accept_delegation(d0).unwrap();
    state.accept_delegation(d1).unwrap();

    assert_eq!(
        state.accept_delegation(d0),
        Err(IdentityError::InvalidAuthorityEpoch)
    );
    assert_eq!(state.accepted_delegation_epoch(AuthorityRole::DeviceSigning).unwrap(), Some(5));
    assert_eq!(state.current_delegation(AuthorityRole::DeviceSigning).unwrap().delegated_key_id(), d1.delegated_key_id());
}
```

Also cover: first observed epoch may be greater than zero; wrong owner; wrong role; wrong root/signature; alternate delegation at equal epoch; role independence; failed replacement leaves state unchanged.

- [ ] **Step 2: Run the RED tests**

Run:

```bash
cargo test -p crosslab-identity --test authority_state
```

Expected: compile failure because `OwnerAuthorityState` does not exist yet.

- [ ] **Step 3: Implement the smallest state object**

Use one private focused slot type, not a `HashMap` or trait hierarchy:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct DelegatedRoleState {
    accepted_epoch: Option<u64>,
    active: Option<AuthorityDelegation>,
}
```

`accept_delegation` must validate against `self.root` before mutation, then compare `delegation_epoch()` strictly against the retained floor. Do not compute `floor + 1`; compare with `<=` so `u64::MAX` fails cleanly without overflow.

- [ ] **Step 4: Add root-successor RED/GREEN coverage**

Add:

```rust
#[test]
fn root_successor_clears_active_roles_but_keeps_epoch_floors() {
    // accept DeviceSigning epoch 7 under root 0
    // accept dual-signed root successor to root 1
    // current_delegation(DeviceSigning) => UnknownIssuer
    // accepted_delegation_epoch(DeviceSigning) => Some(7)
    // epoch 7 under new root => InvalidAuthorityEpoch
    // epoch 8 under new root => accepted
}
```

Implement `accept_root_successor` as validate-then-commit: `successor.verify(&self.root)?`, then replace root, clear only active delegation objects, retain floors.

- [ ] **Step 5: Run focused identity verification**

```bash
cargo fmt --check
cargo test -p crosslab-identity --all-features
cargo clippy -p crosslab-identity --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Commit**

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
- Preserve unchanged bytes in: `crates/identity/tests/golden_vectors.rs`

**Interfaces:**
- Consumes: `OwnerAuthorityState::root()` and `current_delegation(AuthorityRole::DeviceSigning)`.
- Produces final current-authority methods:

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

Keep existing explicit root/delegation methods only temporarily while downstream crates migrate; Task 7 removes them from the public high-level surface.

- [ ] **Step 1: Add a RED stale-Device-Signing credential test**

```rust
#[test]
fn superseded_device_signing_authority_cannot_issue_or_verify_current_credentials() {
    // state accepts DSA epoch 0, issue credential
    // state accepts DSA epoch 1
    // old issuer key cannot issue through issue_current
    // old credential fails verify_current under current authority
}
```

Expected failure before implementation: no `*_current` methods.

- [ ] **Step 2: Implement current-authority credential helpers**

Each method resolves the active Device Signing delegation from `OwnerAuthorityState`; callers never pass a delegation floor. `issue_current` verifies that `issuer_key.verifying_key()` matches the active delegation. `verify_current` verifies the credential's `issuer_device_signing_key_id` matches the active delegation exactly.

- [ ] **Step 3: Preserve credential-epoch rules**

Do not change initial credential epoch `0`, ordinary device-key rotation exact `N+1`, signed transcript fields, key derivation, or signature bytes.

- [ ] **Step 4: Run focused identity tests including vectors**

```bash
cargo test -p crosslab-identity --all-features
```

Expected: all identity and golden-vector tests PASS.

- [ ] **Step 5: Commit**

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
- Preserve: `crates/policy/tests/golden_vectors.rs`

**Interfaces:**
- Owner approval resolves `AuthorityRole::Administrative` from `OwnerAuthorityState`.
- Pairing/credential rotation resolves current `DeviceSigning` authority.
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

Add a policy test where Administrative epoch 0 signs approval evidence, state accepts Administrative epoch 1, then `verify_current` of epoch-0 evidence fails and epoch-0 issuer key cannot create new accepted current evidence.

- [ ] **Step 2: Write RED root/delegated revocation-currentness tests**

Add tests proving:

```rust
// old root-signed revocation cannot apply after state accepts RootSuccessor
// old delegated Administrative/DeviceSigning revocation cannot apply after newer role delegation
// Recovery delegation remains rejected for ordinary revocation
```

- [ ] **Step 3: Write RED credential-rotation/pairing currentness tests**

Prove `TrustRecord::accept_successor_credential_current`, `PairingTrustTransition::issue_current`, and `establish_current` reject a credential or transition tied only to a superseded Device Signing delegation.

- [ ] **Step 4: Implement current-authority policy methods**

Route verification through `OwnerAuthorityState`; remove caller-selected `minimum_delegation_epoch` from all new current-authority methods. Keep policy evaluation pure: `PolicyState::evaluate` itself must not gain key or authority mutation behavior.

- [ ] **Step 5: Run policy tests/vectors**

```bash
cargo fmt --check
cargo test -p crosslab-policy --all-features
cargo clippy -p crosslab-policy --all-targets --all-features -- -D warnings
```

Expected: PASS, golden bytes unchanged.

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
- Add/modify simulator lifecycle tests under `apps/sim/tests/`
- Migrate session fixtures in `transports/quic/src/session_auth_tests.rs` and `transports/quic/src/session_auth_tests/lifecycle_tests.rs` without changing Quinn semantics.

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

`LogicalSession` adds:

```rust
pub fn revalidate_authority(
    &mut self,
    authority: &OwnerAuthorityState,
) -> Result<(), SessionError>;
```

Use focused errors such as `OwnerAuthorityChanged` and `DeviceSigningAuthorityChanged`; on either error, close fail-closed just like peer-currentness revalidation.

- [ ] **Step 1: Write RED fresh-session stale-authority tests**

In `session_authority_currentness.rs`, build state with DSA epoch 0, credentials signed by epoch 0, then accept DSA epoch 1 before authentication. `session.authenticate(...)` must fail before `Active`.

Also accept a root successor, leaving Device Signing inactive, and prove fresh ordinary authentication fails until a strictly newer Device Signing delegation is accepted under the new root.

- [ ] **Step 2: Write RED active-session authority-currentness tests**

```rust
#[test]
fn device_signing_rotation_closes_active_session() {
    // authenticate while DSA epoch 0 is current
    // accept valid DSA epoch 1 locally
    // revalidate_authority -> DeviceSigningAuthorityChanged
    // session state == Closed
}
```

Add equivalent root-successor test and negative tests showing Administrative-only and Recovery-only rotation leave ordinary session current.

- [ ] **Step 3: Migrate pairing flow**

Change inviter/joiner APIs to take `&OwnerAuthorityState`; remove raw root/delegation/minimum-epoch inputs. Use `DeviceCredential::*_current` and policy `*_current` APIs. Keep pairing transcript/confirmation/credential-acceptance bytes unchanged.

- [ ] **Step 4: Migrate fresh session authentication**

Resolve root and current Device Signing delegation from state, call `verify_current`, and snapshot root/DSA key IDs + epochs into `SessionContext`. Keep `SessionAuthTranscriptV1` inputs/bytes unchanged.

- [ ] **Step 5: Implement `revalidate_authority`**

Compare the context snapshot to current `authority.root()` and `authority.current_delegation(DeviceSigning)`. Missing current Device Signing authority counts as stale. On mismatch, use the existing close path and return the focused typed error.

- [ ] **Step 6: Propagate lifecycle cancellation in simulator runtimes**

Add methods equivalent to:

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

`SimStreamRuntime` must additionally call its existing session-authority cancellation helper so active operations/streams are cancelled before transport close.

- [ ] **Step 7: Migrate Quinn/session fixtures**

Construct one `OwnerAuthorityState` per owner fixture and pass it to `SessionActivation`; do not move identity authority into Quinn types.

- [ ] **Step 8: Run focused core/simulator/Quinn tests**

```bash
cargo test -p crosslab-core --all-features
cargo test -p crosslab-sim --all-features
cargo test -p crosslab-transport-quic --all-features
```

Use the actual Quinn package name from `transports/quic/Cargo.toml` if it differs from `crosslab-transport-quic`.

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
- Modify `crates/core/src/control/mod.rs` only if tests expose a mismatch.

**Interfaces:**
- No wire/API change is expected.
- Existing `ControlDispatcher` bounded `inbound_requests`, completed set/order, and directional `ControlSequence` remain the implementation basis.

- [ ] **Step 1: Add an aged-out-ID characterization test**

Use `state_capacity = 2`; accept/complete request A, accept/complete B, accept/complete C so A is evicted from recent completed history, then send A again with the next valid `message_seq`.

Expected: A is treated as a new request attempt, not `DuplicateRequest`.

- [ ] **Step 2: Prove current authorization still applies after eviction**

Repeat the aged-out A attempt but pass a fresh `PolicyState::new()` with no matching allow rule.

Expected:

```rust
Err(ControlDispatchError::AuthorizationDenied(
    DecisionReason::NoMatchingRule
))
```

This proves history eviction does not create authority.

- [ ] **Step 3: Preserve exact-envelope replay protection**

Add/retain a test that reuses an already consumed old `message_seq` and expect `ControlDispatchError::Sequence(...)` before request execution, independent of `RequestId` history.

- [ ] **Step 4: Run the characterization suite**

```bash
cargo test -p crosslab-core --test control_replay
```

Expected: PASS on current bounded implementation. If any test fails, make only the smallest change in `control/mod.rs` required by ADR-0011; do not add an unbounded history set or generic result cache.

- [ ] **Step 5: Commit**

```bash
git add crates/core/tests/control_replay.rs crates/core/src/control/mod.rs
git commit -m "test(core): lock bounded request replay semantics"
```

---

### Task 6: Derive stream currentness from local trust/policy state

**Files:**
- Modify: `crates/core/src/stream/mod.rs`
- Modify: `crates/core/tests/stream_admission.rs`
- Modify: `apps/sim/src/stream.rs`
- Modify: simulator stream tests under `apps/sim/tests/`

**Interfaces:**

Replace caller-selected numeric currentness:

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

Before calling the low-level `AuthorizedOperation::reserve_stream_use`, validate that `peer_trust` matches the authenticated owner/device and is currently `Trusted`; derive `peer_trust.trust_revision()` and `policy.revision()` internally.

Add focused errors:

```rust
PeerTrustMismatch,
PeerNotTrusted,
```

Do not expose a second high-level overload that accepts raw revision numbers.

- [ ] **Step 1: Rewrite stream tests to the desired API and capture RED**

Change the fixture to retain actual `TrustRecord` and `PolicyState`, not copied revision integers. Add:

```rust
#[test]
fn revoked_or_mismatched_peer_trust_cannot_be_hidden_by_revision_inputs() {
    // current API should no longer permit supplying hand-picked revision u64 values
    // wrong TrustRecord identity => PeerTrustMismatch
    // locally revoked trust => PeerNotTrusted or revision invalidation, before admission
}
```

Run:

```bash
cargo test -p crosslab-core --test stream_admission
```

Expected: compile failure until `admit_inbound` is changed.

- [ ] **Step 2: Implement local-state derivation**

Import `PolicyState`, `TrustRecord`, and `TrustState`; validate trust identity/state against `SessionContext`; pass only locally derived revisions into `reserve_stream_use`.

Keep `AuthorizedOperation::validate/reserve_stream_use` explicit-revision methods as lower-level policy primitives for focused policy tests.

- [ ] **Step 3: Migrate `SimStreamRuntime::accept_one`**

Use:

```rust
pub fn accept_one(
    &mut self,
    now: u64,
    peer_trust: &TrustRecord,
    policy: &PolicyState,
) -> Result<StreamId, SimStreamError>;
```

No simulator caller may declare current revision numbers.

- [ ] **Step 4: Run focused stream tests**

```bash
cargo test -p crosslab-core --test stream_admission
cargo test -p crosslab-sim --all-features
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/stream crates/core/tests/stream_admission.rs apps/sim
git commit -m "fix(core): derive stream currentness from local state"
```

---

### Task 7: Remove obsolete caller-selected authority APIs and verify compatibility

**Files:**
- Modify: `crates/identity/src/credential.rs`
- Modify: `crates/policy/src/authorization/approval.rs`
- Modify: `crates/policy/src/trust/{mod.rs,pairing.rs,transition.rs`
- Update remaining test/fixture call sites across `crates/`, `apps/sim/`, and `transports/quic/`.
- Preserve golden/vector files except mechanical fixture construction.

**Interfaces:**
- Remove ordinary public methods that accept raw root + delegation + caller-selected delegation floor where a current-authority method now exists.
- Keep `AuthorityDelegation::verify(root, minimum_epoch)` as low-level cryptographic/import validation only.
- Keep raw import constructors such as `DeviceCredential::from_unverified_signed_parts` where they do not establish currentness.

- [ ] **Step 1: Remove legacy high-level raw-currentness methods**

Delete or make non-public the old `DeviceCredential::{issue,issue_for_public_key,verify,rotate}` and corresponding policy methods that let callers select current delegation/root/floor directly.

- [ ] **Step 2: Use compiler errors as the migration checklist**

Run:

```bash
cargo check --workspace --all-targets --all-features
```

Expected initially: compile errors at any remaining legacy call sites. Migrate every production/test fixture to `OwnerAuthorityState`; do not add compatibility shims that reintroduce caller-selected currentness.

- [ ] **Step 3: Verify no production caller-selected delegation floors remain**

Search tracked Rust source for `minimum_delegation_epoch` and for `credential.verify(` forms that still take explicit root/delegation currentness. The only permitted raw epoch-floor usage is inside low-level delegation cryptographic verification/tests that do not claim local currentness.

- [ ] **Step 4: Re-run all cryptographic/protocol golden vectors**

```bash
cargo test -p crosslab-identity --test golden_vectors
cargo test -p crosslab-policy --test golden_vectors
cargo test -p crosslab-protocol --test wire_vectors
```

Expected: byte-for-byte existing vectors PASS unchanged.

- [ ] **Step 5: Commit**

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
- Modify implementation-plan checkbox state only with actual evidence; do not mark unrun checks complete.

**Interfaces:**
- No new runtime interface. This task proves the accepted foundation implementation and records exact evidence.

- [ ] **Step 1: Run complete local/workspace verification**

```bash
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
```

Expected: all commands PASS. The already documented `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning may remain only if still isolated to the Iroh experiment and `cargo audit` exits successfully under the repository's accepted audit configuration.

- [ ] **Step 2: Run bounded fuzz smoke for affected parser/security surfaces**

```bash
cargo +nightly-2026-09-12 fuzz run control_frame --manifest-path fuzz/Cargo.toml -- -runs=256
cargo +nightly-2026-09-12 fuzz run data_stream_open --manifest-path fuzz/Cargo.toml -- -runs=256
cargo +nightly-2026-09-12 fuzz run identifiers --manifest-path fuzz/Cargo.toml -- -runs=256
cargo +nightly-2026-09-12 fuzz run pairing_bootstrap --manifest-path fuzz/Cargo.toml -- -runs=256
cargo +nightly-2026-09-12 fuzz run session_auth --manifest-path fuzz/Cargo.toml -- -runs=256
```

If the local cargo-fuzz CLI requires the repository workflow invocation form, use exactly the `.github/workflows/fuzz.yml` commands instead; do not change fuzz semantics merely to make a command line convenient.

- [ ] **Step 3: Reconcile the security assessment**

Record:

- ADR-0010 authority currentness resolved;
- old root/delegated authority no longer accepted by ordinary high-level paths;
- root/Device Signing active-session currentness behavior verified;
- ADR-0011 bounded request semantics aligned in spec/tests;
- stream currentness derives from local trust/policy state;
- no wire/golden change;
- remaining deferred risks: production authority persistence/rollback protection, platform key stores, system-event family authorization when introduced, branch protection, platform CI, isolated Iroh `paste` warning if still present.

- [ ] **Step 4: Update `CURRENT.md` with exact commit/workflow evidence**

The exact next task becomes **M9 Task 4 remote-networking architecture work** only after the implementation PR is merged and post-merge `main` verification is green. Until then, keep M9 Task 4 blocked.

- [ ] **Step 5: Push and verify exact-head CI/Fuzz**

Push the implementation branch and require:

- Rust CI exact-head success for audit/fmt/check/Clippy/tests;
- Fuzz Smoke exact-head success when triggered by affected paths;
- PR diff contains no unintended protocol/schema/generated-byte changes.

- [ ] **Step 6: Commit the final reconciliation**

```bash
git add security docs/development/CURRENT.md docs/plans/phase-1/M9-whole-project-security-quality-audit.md docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md
git commit -m "docs: reconcile final M9 foundation hardening gate"
```

- [ ] **Step 7: Merge only after exact-head evidence is green**

After merge, verify the `main` merge commit with the full Rust CI gate. Only then update/confirm `CURRENT.md` so M9 Task 4 is unblocked.
