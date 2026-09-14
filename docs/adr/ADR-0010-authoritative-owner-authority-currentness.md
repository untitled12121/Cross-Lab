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
  device_signing: optional AuthorityDelegation
  administrative: optional AuthorityDelegation
  recovery: optional AuthorityDelegation
```

The state stores the exact currently accepted authority object for each slot, not only an epoch floor.

`OwnerAuthorityState` belongs in the identity domain. It must not depend on networking, protocol, UI, platform-adapter, or persistence types.

### 2. Active root currentness

The state owns the active `OwnerRootRecord`.

A normal root successor may replace it only after the existing `RootSuccessor` continuity rules verify the exact owner, expected next root epoch, old-root signature, new-root signature, and successor key identity.

After a successor is accepted, high-level ordinary owner-authority operations must no longer accept the superseded root as current authority.

Raw root-record cryptographic verification may remain available for import/vector/low-level validation, but it does not by itself establish production currentness.

Emergency root recovery remains outside this Phase 1 decision.

### 3. Delegated-role currentness

For `DeviceSigning`, `Administrative`, and `Recovery` role slots:

- a first accepted delegation must verify under the active root, match the owner and role, and carry a valid signature;
- a first locally observed delegation may have any valid epoch because a restored or newly joined device may first observe the current role at epoch greater than zero;
- replacement must verify completely before mutation;
- replacement requires a strictly greater role-specific delegation epoch;
- an equal, lower, wrong-owner, wrong-role, wrong-root, invalid-signature, or otherwise ambiguous replacement fails closed;
- once epoch `N+1` is accepted, epoch `N` cannot authorize new sensitive work through high-level APIs.

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

Low-level constructors used for deterministic vectors/import may remain narrower primitives, but they must not be the ordinary production currentness boundary.

### 5. Session consequences of authority change

Cross-Lab chooses a fail-closed Phase 1 rule for identity authority that participated in session authentication:

- accepting a new active owner root invalidates ordinary active sessions authenticated under the superseded root;
- accepting a new `DeviceSigning` delegation invalidates ordinary active sessions authenticated under the superseded Device Signing authority;
- invalidation rejects new control work, cancels session-scoped authorized operations/data-stream authority, and closes the logical session/transport as soon as practical;
- reconnect requires fresh authentication under current authority state;
- rotating `Administrative` or `Recovery` authority alone does not invalidate an ordinary device session because those roles did not authenticate that session.

This rule is intentionally conservative. Phase 1 does not distinguish routine Device Signing/root rotation from compromise-response rotation for session continuation.

Session state therefore needs enough local authentication-currentness metadata to detect root/Device Signing authority replacement without trusting peer-provided authority state.

### 6. Persistence boundary

Phase 1 implementation may keep `OwnerAuthorityState` in memory for simulator/runtime work.

This ADR does not claim crash/restart rollback resistance. Before production platform state relies on this authority across restarts, the local agent/platform persistence layer must atomically persist the active root and delegated-role slots and revalidate them when loading.

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

Rejected as the primary design. Numeric floors would improve stale-epoch rejection but still allow arbitrary authority objects to flow through high-level APIs and require each caller to bind key identity/role/root correctly. Storing the exact accepted authority object is simpler and more misuse-resistant.

### Keep caller-provided root/delegation currentness

Rejected. Cryptographic validity of a supplied historical authority is not equivalent to local current authority after rotation.

### Introduce a persistent authority database now

Rejected for this Phase 1 slice. Cross-Lab does not yet have the production local persistence boundary. Introducing a database here would mix identity semantics with a premature platform/storage subsystem.

### Grandfather active sessions after Device Signing/root rotation

Rejected for Phase 1. It permits authority known to be superseded locally to retain session-scoped ordinary power until another lifecycle event closes the session. A future, explicitly reviewed design may distinguish routine rotation from emergency containment if operational evidence justifies the complexity.

## Security impact

This decision removes caller-selected authority currentness from ordinary sensitive paths. Superseded owner roots and delegated-role keys remain cryptographically historical but cease to be locally current authority after a verified replacement is accepted.

Fail-closed session invalidation prevents an already authenticated session from retaining ordinary authority after the owner root or Device Signing authority that authenticated it has been replaced locally.

Recovery remains a separate role and does not become ordinary device/session authority.

## Compatibility impact

No wire-format, canonical transcript, signature format, identifier format, or protocol version change is required.

The change is an internal Rust API and runtime-authority semantic hardening. Golden cryptographic/protocol vectors should remain byte-for-byte unchanged unless a later separately approved decision says otherwise.

## Operational impact

Phase 1 gains a small identity-owned runtime state object and explicit authority-replacement lifecycle hooks. No new database, daemon, network service, transport dependency, or platform-specific dependency is introduced.

Production persistence, hardware-backed key-provider integration, and emergency root recovery remain later platform/security work.

## Consequences

- ordinary owner-authority currentness has one local source of truth;
- active root and delegated-role replacement become explicit verified state transitions;
- sensitive callers no longer choose their own minimum delegation epoch or current root object;
- root/Device Signing replacement invalidates ordinary sessions authenticated under superseded authority;
- Administrative/Recovery rotation does not unnecessarily tear down ordinary device sessions;
- identity remains transport-independent and persistence-implementation-independent;
- restart rollback resistance remains an explicit later persistence requirement rather than an accidental Phase 1 claim.
