# Authoritative Delegated-Role Currentness Design

**Date:** 2026-09-14  
**Status:** Proposed design for ADR-0010  
**Scope:** Foundation remediation only; no M9 remote-networking behavior change

## Goal

Make delegated owner-authority currentness explicit, local, typed, and fail-closed so that a stale but historically valid delegation cannot authorize credential, trust, approval, pairing, or session operations after a newer delegation for that role has been accepted locally.

This design implements the semantic requirement already stated in `docs/architecture/IDENTITY-AND-KEYS.md`: older delegated-role epochs must be rejected after newer authority has been accepted into authoritative local state.

## Current problem

`AuthorityDelegation::verify(root, minimum_epoch)` can validate signature, owner, role constraints, and a caller-supplied epoch floor. The floor is not itself authoritative state.

High-level call paths currently recreate currentness differently:

- device credential APIs accept a raw delegation and minimum delegation epoch;
- pairing trust establishment accepts a raw issuer and minimum delegation epoch;
- owner approval evidence accepts an Administrative delegation plus minimum delegation epoch;
- delegated trust transitions accept a raw delegation plus minimum delegation epoch;
- session authentication verifies each supplied credential issuer using that same issuer's own `delegation_epoch()` as its minimum.

The last case demonstrates the misuse risk directly: a stale delegation can satisfy the floor derived from itself.

## Architecture

### Identity-owned state

Add a focused identity-domain module, expected shape:

```text
crates/identity/src/authority_state.rs
```

with a public state object conceptually equivalent to:

```rust
pub struct OwnerAuthorityState {
    root: OwnerRootRecord,
    device_signing: Option<AuthorityDelegation>,
    administrative: Option<AuthorityDelegation>,
    recovery: Option<AuthorityDelegation>,
}
```

The implementation may use a small private helper to map `AuthorityRole` to the three delegated slots. It should not introduce a generic hashmap, global registry, trait hierarchy, service object, or persistence abstraction before one is needed.

`AuthorityRole::OwnerRoot` is not a delegated slot and remains represented by `OwnerRootRecord`.

### Responsibilities

`OwnerAuthorityState` is responsible for:

- exposing the active owner/root identity;
- accepting an initial delegated role only after root signature/owner/role verification;
- replacing an accepted delegated role only with a strictly higher epoch;
- rejecting equal/lower epochs;
- returning the currently accepted delegation for a requested role;
- accepting a verified `RootSuccessor` transition;
- clearing delegated-role slots after root succession.

It is not responsible for:

- private-key storage;
- transport identity;
- policy decisions;
- UI state;
- network synchronization;
- persistence engine selection;
- cross-device conflict resolution.

## API direction

Exact names may change during implementation if tests expose a cleaner shape, but the authority boundary should remain equivalent to:

```rust
impl OwnerAuthorityState {
    pub fn new(root: OwnerRootRecord) -> Self;

    pub fn root(&self) -> &OwnerRootRecord;

    pub fn accept_delegation(
        &mut self,
        delegation: AuthorityDelegation,
    ) -> Result<(), IdentityError>;

    pub fn current_delegation(
        &self,
        role: AuthorityRole,
    ) -> Result<&AuthorityDelegation, IdentityError>;

    pub fn accept_root_successor(
        &mut self,
        successor: &RootSuccessor,
    ) -> Result<(), IdentityError>;
}
```

`accept_delegation` may accept by value so the state owns the exact accepted signed object and callers cannot mutate/replace it behind the state's back.

A dedicated `AuthorityStateError` should be introduced only if `IdentityError` cannot express the required failures clearly without becoming ambiguous. Prefer the smallest typed error surface.

## Delegation acceptance rules

For a requested delegation:

1. reject `AuthorityRole::OwnerRoot` as a delegated role;
2. require `delegation.owner_id() == state.root().owner_id()`;
3. verify the delegation signature against the active root;
4. locate the exact role slot from the delegation's signed role;
5. if the slot is empty, accept the delegation as this local state's current anchor;
6. if populated, require `new_epoch > current_epoch`;
7. commit the replacement only after every validation succeeds.

Failure must leave the existing state unchanged.

The design intentionally does not require `new_epoch == current_epoch + 1`; the architecture requires monotonic delegated-role epochs, not contiguous history.

## Root succession

`OwnerAuthorityState::accept_root_successor` verifies the existing `RootSuccessor` against the current root using the already-defined dual-signature continuity rules.

On success it atomically:

1. replaces the root with the verified successor root;
2. clears Device Signing, Administrative, and Recovery delegated slots.

No old-root delegation remains current after root replacement. Re-establishing delegated roles requires new delegations signed by the new root.

Failure leaves the root and all delegated slots unchanged.

Emergency root recovery remains outside Phase 1.

## High-level API migration

### Identity credential APIs

`DeviceCredential` issuance, verification, and rotation should stop accepting caller-selected delegation epoch floors for production authority checks.

They should resolve the current `DeviceSigning` delegation from `OwnerAuthorityState` and then verify issuer-key identity/signature as they do today.

Low-level import/vector helpers may continue to operate on explicit signed objects where deterministic testing requires it, but ordinary production paths must not rely on caller-supplied currentness.

### Pairing and trust establishment

`PairingInviterFlow` and `PairingTrustTransition` should use the authority state for Device Signing currentness.

The joiner credential acceptance path must also verify the credential through the same current state rather than using a literal `0` or the supplied issuer's own epoch as delegation currentness.

### Policy approvals

`OwnerApprovalEvidence::{issue, verify}` should resolve the current `Administrative` delegation from `OwnerAuthorityState`.

A caller may still provide the corresponding private signing key when issuing evidence; the key must match the currently accepted Administrative delegation.

### Delegated trust transitions

Delegated revocation issuance/application should resolve the signed issuer role against the current authority state. Existing role restrictions remain unchanged: ordinary delegated revocation continues to permit only the roles already allowed by policy/trust semantics.

Root-authorized revocation remains a separate root path.

### Session authentication

`SessionActivation` should receive authoritative owner state rather than only an owner root when delegated credential issuers must be checked.

Both initiator and responder credentials must verify through the current Device Signing delegation accepted in that state.

A stale credential issuer delegation must fail before the session becomes Active, even if its signature remains cryptographically valid.

Session transcript bytes, proof bytes, channel binding, negotiated protocol/features, and `SessionId` derivation remain unchanged.

## Ownership and layering

Dependency direction stays unchanged:

```text
crosslab-crypto
      ↑
crosslab-identity   <- owns OwnerAuthorityState
      ↑
crosslab-policy     <- consumes current authority for approval/trust
      ↑
crosslab-core       <- orchestrates pairing/session lifecycle
```

`crosslab-identity` must not depend on Core, Policy, transports, platform adapters, or UI.

Core may own an `OwnerAuthorityState` instance as part of runtime owner-domain lifecycle later, but this remediation should avoid inventing a large owner service or daemon merely to carry it.

## Persistence and rollback

This milestone deliberately does not add a persistence backend or storage schema.

Phase 1 simulator/core tests may use in-memory state. Production platform work must eventually persist:

- active `OwnerRootRecord`;
- currently accepted Device Signing delegation, if present;
- currently accepted Administrative delegation, if present;
- currently accepted Recovery delegation, if present.

That future persistence must preserve atomic replacement and defend against rollback appropriate to each platform. Until then, this remediation guarantees currentness only relative to the authoritative local state currently loaded in memory.

A fresh/empty state cannot infer that a valid historical delegation is globally newest. Initial authority state therefore must come from a trusted owner-domain bootstrap, recovery, or durable restore path; root signature plus epoch magnitude alone is not proof of global freshness.

Generic CRDT merge, last-write-wins merge, peer gossip, transport discovery, or remote claims must never decide security-current authority state.

## Error handling

Required fail-closed categories include:

- wrong owner;
- delegated OwnerRoot role;
- invalid root signature/unknown issuer;
- stale/equal delegated epoch;
- missing required current role;
- issuer key mismatch;
- invalid root successor.

State mutation must occur only after validation succeeds. Tests should assert unchanged state after failed replacements/root transitions.

## Regression-first implementation sequence

1. Add identity RED tests proving a newer accepted role delegation does not currently make the old delegation unusable through existing high-level APIs.
2. Add `OwnerAuthorityState` with focused unit tests for role isolation, first acceptance, strictly-higher replacement, stale/equal rejection, wrong owner/role/signature, and failure atomicity.
3. Migrate `DeviceCredential` issuance/verification/rotation to current Device Signing state.
4. Migrate pairing and pairing trust establishment.
5. Migrate Administrative approval evidence.
6. Migrate delegated trust transitions/revocation.
7. Migrate session authentication and add a lifecycle regression: authenticate using an issuer, advance accepted Device Signing delegation, then prove the old issuer cannot establish a fresh session.
8. Add root-successor tests proving successful transition clears all delegated slots and failed transition changes nothing.
9. Remove production call-site use of arbitrary `minimum_delegation_epoch` values where currentness is required.
10. Run full fmt/check/Clippy/tests, protocol/golden regressions, fuzz smoke, and dependency audit on the exact final head.

Keep commits small enough that each authority-boundary migration is reviewable independently.

## Required tests

At minimum:

- first valid Device Signing delegation is accepted;
- higher Device Signing epoch replaces it;
- equal/lower Device Signing epochs fail;
- Administrative and Recovery epochs are independent;
- wrong-owner delegation fails without mutation;
- OwnerRoot-as-delegation fails;
- bad root signature fails without mutation;
- stale Device Signing authority cannot issue a new credential after rotation;
- stale Device Signing authority cannot validate a credential as current after rotation;
- pairing cannot establish trust using stale Device Signing authority;
- stale Administrative authority cannot issue/verify current owner approval evidence;
- stale delegated revocation authority cannot revoke after its role rotates;
- fresh session authentication rejects credentials whose issuer delegation is no longer current;
- successful root successor clears all delegated slots;
- failed root successor preserves root and delegated slots;
- existing canonical credential, pairing, session-auth, and protocol golden vectors remain unchanged;
- no new secret/debug logging exposure is introduced.

## Non-goals

This work does not:

- implement emergency root recovery;
- select persistent storage;
- define authority synchronization over the network;
- add a cloud coordinator;
- change M9 transport selection;
- change credential or delegation wire/signing formats;
- create a general key-management daemon;
- solve storage rollback before platform persistence exists.

## Completion criteria

The remediation is complete only when:

- ADR-0010 is accepted;
- the regression-first implementation removes caller-selected currentness from production authority-bearing paths;
- stale delegated roles are rejected consistently across credential, pairing, policy/trust, and session boundaries;
- exact-head CI and fuzz are green;
- `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md` and `docs/development/CURRENT.md` are reconciled;
- the final M9 foundation hardening gate passes before M9 Task 4 resumes.
