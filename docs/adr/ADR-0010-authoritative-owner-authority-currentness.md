# ADR-0010: Authoritative owner authority currentness

**Status:** Proposed  
**Date:** 2026-09-14

## Context

Cross-Lab identity already defines monotonically advancing owner-root and delegated-role epochs. The focused identity specification requires consumers to reject older delegated-role epochs after a newer delegation is accepted into authoritative local state.

The current Phase 1 APIs do not yet provide one authoritative owner-domain currentness boundary. High-level credential, pairing, approval, trust-transition, and session APIs can receive raw `OwnerRootRecord`, `AuthorityDelegation`, and caller-selected minimum delegation epochs. This creates a misuse class where an otherwise valid but superseded authority object can satisfy its own currentness check.

The same issue exists at the owner-root layer. Root-authorized trust transitions currently accept whichever `OwnerRootRecord` the caller supplies. After a valid root successor is accepted locally, possession of the superseded root key must not remain ordinary current owner authority merely because a caller can still supply the old root record.

This is an identity/trust architecture decision. It must remain independent of transport identity, networking candidates, CRDT state, UI state, and future persistence implementation details.

## Decision

### 1. Introduce owner-domain authoritative runtime state

The identity domain will expose an `OwnerAuthorityState` that is the authoritative in-process source for current owner authority:

```text
OwnerAuthorityState
  active_root: OwnerRootRecord
  device_signing: DelegatedRoleState
  administrative: DelegatedRoleState
  recovery: DelegatedRoleState

DelegatedRoleState
  accepted_epoch: optional u64
  active: optional AuthorityDelegation
```

For each delegated role, the state retains the monotonic last accepted epoch and, when one is valid under the active root, the exact active delegation object. This keeps role/key/root binding explicit while preventing role-epoch rollback across owner-root rotation.

`OwnerAuthorityState` belongs in the identity domain. It must not depend on networking, protocol, UI, platform-adapter, or persistence types.

### 2. Active root currentness

The state owns the active `OwnerRootRecord`.

A normal root successor may replace it only after the existing `RootSuccessor` continuity rules verify the exact owner, expected next root epoch, old-root signature, new-root signature, and successor key identity.

Root replacement is a validate-then-commit state transition. When the successor becomes active:

- the previous root becomes historical rather than current ordinary authority;
- every delegated-role `active` slot is cleared because those delegations were authorized by the superseded root;
- each delegated role retains its `accepted_epoch` floor;
- a new delegation under the new root must verify under that root and strictly advance the retained role epoch before becoming active.

This prevents root rotation from silently carrying old-root delegations forward or resetting role epochs backward.

After a successor is accepted, high-level ordinary owner-authority operations must no longer accept the superseded root as current authority.

Raw root-record cryptographic verification may remain available for import/vector/low-level validation, but it does not by itself establish production currentness.

Emergency root recovery remains outside this Phase 1 decision.

### 3. Delegated-role currentness

For `DeviceSigning`, `Administrative`, and `Recovery` role slots:

- when no prior role epoch is known, the first accepted delegation must verify under the active root, match the owner and role, and carry a valid signature; it may begin at any valid epoch because a restored or newly joined device may first observe the current role at epoch greater than zero;
- when a prior role epoch is known, replacement must verify completely before mutation and must carry a strictly greater role-specific delegation epoch;
- an equal, lower, wrong-owner, wrong-role, wrong-root, invalid-signature, or otherwise ambiguous replacement fails closed;
- once epoch `N+1` is accepted, epoch `N` cannot authorize new sensitive work through high-level APIs;
- if root rotation clears the active delegation, the retained epoch floor still prevents a new root from accepting role epoch `N` or lower after role epoch `N+1` had already been accepted.

Raw `AuthorityDelegation::verify(root, minimum_epoch)` may remain as a low-level validation primitive, but caller-selected epoch floors are not authoritative currentness.

### 4. High-level authority-bearing APIs consume authoritative state

Sensitive high-level APIs must obtain current authority from `OwnerAuthorityState` rather than accept caller-selected currentness values.

At minimum:

- device credential issuance, verification, and rotation select the current `DeviceSigning` delegation from state;
- pairing credential issuance/acceptance and pairing trust establishment use the current `DeviceSigning` authority;
- owner approval issuance/verification uses the current `Administrative` authority;
- delegated trust revocation uses the current accepted delegation for the signed allowed role;
- root-authorized trust transitions use the active root from state;
- session authentication validates both device credentials against the state's current `DeviceSigning` authority.

A missing active role delegation fails closed even if the state retains a historical accepted epoch.

Low-level constructors used for deterministic vectors/import may remain narrower primitives, but they must not be the ordinary production currentness boundary.

### 5. Session consequences of authority change

Cross-Lab chooses a fail-closed Phase 1 rule for identity authority that participated in session authentication:

- accepting a new active owner root invalidates ordinary active sessions authenticated under the superseded root;
- accepting a new `DeviceSigning` delegation invalidates ordinary active sessions authenticated under the superseded Device Signing authority;
- root replacement also leaves Device Signing inactive until a strictly newer delegation under the new root is accepted, so fresh ordinary session authentication fails closed during that interval;
- invalidation rejects new control work, cancels session-scoped authorized operations/data-stream authority, and closes the logical session/transport as soon as practical;
- reconnect requires fresh authentication under current authority state;
- rotating `Administrative` or `Recovery` authority alone does not invalidate an ordinary device session because those roles did not authenticate that session.

This rule is intentionally conservative. Phase 1 does not distinguish routine Device Signing/root rotation from compromise-response rotation for session continuation.

Session state therefore needs enough local authentication-currentness metadata to detect root/Device Signing authority replacement without trusting peer-provided authority state.

### 6. Persistence boundary

Phase 1 implementation may keep `OwnerAuthorityState` in memory for simulator/runtime work.

This ADR does not claim crash/restart rollback resistance. Before production platform state relies on this authority across restarts, the local agent/platform persistence layer must atomically persist the active root, each role's monotonic accepted epoch, and each active delegated-role object, then revalidate them when loading.

Owner authority currentness is security state and must not be established by CRDT merge, peer-majority state, relay state, or transport metadata.

### 7. Failure semantics

Missing, conflicting, ambiguous, stale, or unverifiable owner authority state fails closed.

A caller cannot widen authority by supplying:

- a lower minimum epoch;
- a stale delegation object;
- a stale root record;
- a larger unverified epoch number;
- peer or transport metadata claiming authority currentness.

## Alternatives considered

### Store only accepted epoch floors

Rejected as the primary design. Numeric floors are necessary to preserve monotonicity across root rotation, but they are insufficient by themselves because arbitrary authority objects could still flow through high-level APIs. Cross-Lab stores the floor plus the exact active authority object.

### Keep caller-provided root/delegation currentness

Rejected. Cryptographic validity of a supplied historical authority is not equivalent to local current authority after rotation.

### Carry old delegated-role authority across root rotation

Rejected for Phase 1. A delegation signed by the superseded root remains historical evidence, not current delegated authority under the new active root. Carrying it forward would require an explicit cross-root delegation-continuity rule that does not currently exist.

### Reset delegated-role epochs when the root rotates

Rejected. Role epochs are monotonic security currentness for the logical role slot. Root rotation must not create a path to accept an older role epoch.

### Introduce a persistent authority database now

Rejected for this Phase 1 slice. Cross-Lab does not yet have the production local persistence boundary. Introducing a database here would mix identity semantics with a premature platform/storage subsystem.

### Grandfather active sessions after Device Signing/root rotation

Rejected for Phase 1. It permits authority known to be superseded locally to retain session-scoped ordinary power until another lifecycle event closes the session. A future, explicitly reviewed design may distinguish routine rotation from emergency containment if operational evidence justifies the complexity.

## Security impact

This decision removes caller-selected authority currentness from ordinary sensitive paths. Superseded owner roots and delegated-role keys remain cryptographically historical but cease to be locally current authority after a verified replacement is accepted.

Retaining delegated-role epoch floors across root rotation prevents rollback while clearing active old-root delegations prevents cross-root authority carryover without an explicit continuity rule.

Fail-closed session invalidation prevents an already authenticated session from retaining ordinary authority after the owner root or Device Signing authority that authenticated it has been replaced locally.

Recovery remains a separate role and does not become ordinary device/session authority.

## Compatibility impact

No wire-format, canonical transcript, signature format, identifier format, or protocol version change is required.

The change is an internal Rust API and runtime-authority semantic hardening. Golden cryptographic/protocol vectors should remain byte-for-byte unchanged unless a later separately approved decision says otherwise.

## Operational impact

Phase 1 gains a small identity-owned runtime state object and explicit authority-replacement lifecycle hooks. Root rotation temporarily leaves delegated roles inactive until strictly newer delegations are accepted under the new root.

No new database, daemon, network service, transport dependency, or platform-specific dependency is introduced.

Production persistence, hardware-backed key-provider integration, and emergency root recovery remain later platform/security work.

## Consequences

- ordinary owner-authority currentness has one local source of truth;
- active root and delegated-role replacement become explicit verified state transitions;
- delegated-role epoch monotonicity survives root rotation without carrying old-root delegated authority forward;
- sensitive callers no longer choose their own minimum delegation epoch or current root object;
- root/Device Signing replacement invalidates ordinary sessions authenticated under superseded authority;
- Administrative/Recovery rotation does not unnecessarily tear down ordinary device sessions;
- identity remains transport-independent and persistence-implementation-independent;
- restart rollback resistance remains an explicit later persistence requirement rather than an accidental Phase 1 claim.
