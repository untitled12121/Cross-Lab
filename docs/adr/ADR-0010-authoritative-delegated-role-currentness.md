# ADR-0010: Authoritative delegated-role currentness

**Status:** Proposed  
**Date:** 2026-09-14

## Context

Cross-Lab owner authority is role-separated. `DeviceSigning`, `Administrative`, and `Recovery` delegations each carry a role-specific `delegation_epoch` and are signed by the active owner root.

`docs/architecture/IDENTITY-AND-KEYS.md` requires consumers to reject older role epochs after a newer delegation has been accepted into authoritative local state. The current implementation can cryptographically verify a delegation against a caller-supplied minimum epoch, but it does not own an authoritative accepted delegation per role.

This creates a currentness gap. Several authority-bearing APIs accept a raw `AuthorityDelegation` plus a caller-selected minimum epoch. Session authentication is weaker still: it currently verifies a supplied issuer using that same issuer's own epoch as the minimum. A stale but otherwise valid historical delegation can therefore prove its signature and role without proving that local state still accepts it as current.

The gap affects device credential issuance/verification, pairing trust establishment, owner approval evidence, delegated revocation, credential rotation, and session authentication. Fixing only one call path would leave equivalent stale-authority paths elsewhere.

The Master Architecture requires identity/trust semantic changes to be recorded in an ADR and explicitly approved before implementation. ADR-0009 remains reserved for the M9 remote-networking decision.

## Decision

Cross-Lab will introduce an identity-domain `OwnerAuthorityState` as the authoritative local representation of the active owner root and the accepted delegated authority for each ordinary role slot.

Conceptually:

```text
OwnerAuthorityState
  root: OwnerRootRecord
  device_signing: Option<AuthorityDelegation>
  administrative: Option<AuthorityDelegation>
  recovery: Option<AuthorityDelegation>
```

The state belongs in `crosslab-identity`. It is domain state, not transport state, UI state, policy state, or process-global mutable state.

### Delegated-role acceptance

For each delegated role slot:

- the delegation owner must match the state's owner root;
- the delegation role must match the target slot;
- the delegation must verify under the state's active root;
- the first locally accepted delegation establishes the local currentness anchor for that slot;
- replacing an accepted delegation requires a strictly higher role-specific `delegation_epoch`;
- equal or lower epochs are rejected;
- different role slots have independent epoch sequences;
- a delegation accepted for one role cannot satisfy another role.

Phase 1 does not infer global freshness from an epoch number alone. Currentness means the newest delegation that this authoritative local state has accepted. A device restoring or joining an owner domain must obtain its initial authority-state snapshot through an already trusted bootstrap/recovery path. Rollback-resistant durable storage is a later platform/persistence requirement and is not claimed by this ADR.

### Authority-bearing APIs

High-level production APIs will stop treating caller-supplied `minimum_delegation_epoch` values as authority.

Device credential issuance and verification, pairing/trust establishment, approval verification, delegated trust transitions, credential rotation, and session authentication will validate delegated authority through `OwnerAuthorityState` or through a state-derived current-authority reference/capability.

Low-level `AuthorityDelegation` signature verification may remain available for parsing/import/tests, but successful low-level verification alone does not establish current local authority.

Delegated-role currentness is checked when an operation consumes delegated authority. Accepting a newer delegation does not, by itself, retroactively revoke an already Active logical session. Existing session termination semantics remain governed by local trust/revocation and fatal session conditions. A fresh reconnect/session must revalidate credentials through the then-current authority state.

### Root succession

`OwnerAuthorityState` owns the active `OwnerRootRecord`. Normal root succession remains governed by the existing dual-signature `RootSuccessor` continuity contract.

When a root successor is accepted, previously accepted delegated-role slots are cleared. Delegations signed by the previous root do not silently survive the root transition. Each required role must be explicitly reissued and accepted under the new active root.

This is intentionally fail-closed and avoids treating an old-root signature as authority under a new root.

### Persistence boundary

This ADR defines identity-domain semantics only. It does not select a database, serialization format, daemon, sync protocol, or platform storage API.

Phase 1 may keep `OwnerAuthorityState` in memory for simulator/core work. Future platform persistence must durably restore the active root and accepted delegated-role state without weakening currentness. Security-sensitive authority state must not be merged through generic CRDT/eventual-consistency semantics.

## Alternatives considered

### Store only a role-to-minimum-epoch map

Rejected. It is smaller, but high-level callers would still provide arbitrary delegation objects and would need to correctly pair those objects with the numeric floor. Storing the accepted delegation itself makes owner, role, key, signature, and epoch currentness one coherent authority object.

### Keep caller-supplied minimum epochs

Rejected. The current design already demonstrates that call sites can accidentally supply an issuer's own epoch or another non-authoritative value. Currentness is security state and should not depend on every caller recreating the same rule correctly.

### Add a full persistent authority database now

Rejected for this milestone. Persistence is necessary before production platform rollout, but selecting a storage engine and durable schema now would introduce a new subsystem without a current platform consumer. The identity-domain semantics must be correct first and remain storage-adapter independent.

### Use process-global authority state

Rejected. Global mutable security state would make tests, multi-owner operation, lifecycle, recovery, and future platform isolation harder to reason about. Authority state must be explicit in the call graph.

### Require delegated epochs to increment by exactly one

Rejected. The identity specification requires monotonically increasing replacement epochs but does not require contiguous delegated-role history. Requiring exact `N + 1` would invent a stronger compatibility rule than the current specification. Equal/lower epochs are stale; any strictly higher root-signed epoch may replace the locally accepted delegation.

## Security impact

The decision closes the stale delegated-authority gap after local acceptance of a newer role delegation. Historical valid signatures remain cryptographically valid objects but no longer authorize current delegated operations once superseded locally.

The state remains owner-scoped and role-scoped. Transport identifiers, peer claims, UI state, network routes, and capability metadata cannot establish delegated authority.

Root succession invalidates old-root delegated slots, preventing stale old-root delegation reuse after a successful root transition.

A credential whose signature can only be validated through a superseded Device Signing delegation cannot satisfy fresh identity validation after that newer delegation is locally accepted. This is fail-closed and follows the existing identity rule that the issuing Device Signing Authority must be valid for the owner/role/epoch. The remediation does not invent a bulk credential-reissuance protocol.

This ADR does not solve durable-state rollback. If an attacker can restore an older authority-state snapshot, the process can lose knowledge of a newer accepted epoch. Production persistence/recovery work must provide rollback-resistant storage appropriate to each platform before claiming durable currentness across compromise/restart scenarios.

## Compatibility impact

No protocol wire field, canonical transcript, signature format, identifier format, or cryptographic algorithm changes.

Public Rust APIs that currently accept raw delegations plus caller-selected minimum epochs will change. The migration is source-incompatible inside the monorepo but protocol-compatible across peers.

Golden signing/protocol vectors must remain unchanged unless an independently approved protocol change occurs.

## Operational impact

Authority state becomes explicit lifecycle state that Core/platform orchestration must retain alongside owner trust state. The identity crate remains responsible for validation semantics; platform storage adapters later become responsible for durability.

Rotating a delegated role invalidates older local use immediately after acceptance. For Device Signing rotation, credentials that depend only on the superseded signing delegation will not pass a future fresh authentication until the owner domain establishes suitable current credentials under the new authority. The exact owner workflow for bulk reissuance is outside this remediation.

Already Active sessions are not closed solely because a delegated role rotates. Explicit peer revocation/trust transitions remain the mechanism for terminating active peer authority; reconnect always performs fresh credential/trust validation.

Rotating the owner root clears all delegated role slots, so ordinary operations requiring those roles remain unavailable until replacement delegations under the new root are installed.

No new background service, database, network round trip, or dependency is introduced by this decision.

## Consequences

- Delegated-role currentness has one authoritative domain representation instead of caller-selected numeric floors.
- Sensitive call paths fail closed when the required current role is missing or stale.
- Raw `AuthorityDelegation` remains a signed identity object, not proof that the object is currently accepted.
- Identity, policy, pairing, trust, and session code gain an explicit authority-state dependency where they exercise delegated authority.
- Root rotation has explicit delegated-role invalidation semantics.
- Active-session shutdown semantics remain unchanged; fresh authentication uses current authority state.
- Future persistence work has a clear object to store without forcing persistence concerns into the identity model today.
- M9 Task 4 remains blocked until this proposal is accepted, implemented regression-first, and the final foundation hardening gate/reconciliation passes.
