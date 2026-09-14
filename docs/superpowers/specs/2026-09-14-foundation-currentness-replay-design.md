# Foundation Currentness and Replay Design

**Date:** 2026-09-14  
**Status:** Accepted  
**Scope:** M9 second foundation remediation, before remote-networking Task 4

## Purpose

Close the remaining foundation-level currentness and replay ambiguities without introducing platform persistence, new wire formats, or networking-specific authority.

This design is governed by:

- `ADR-0010-authoritative-owner-authority-currentness.md`;
- `ADR-0011-bounded-request-replay-semantics.md`;
- the existing Identity/Keys, Pairing/Trust/Revocation, Policy/Authorization, Session/Transport, Protocol V1, Security Boundaries, and Threat Model specifications.

ADR-0009 remains reserved for the M9 remote-networking architecture decision.

## 1. Authority architecture

### 1.1 Owner authority state

Add an identity-domain `OwnerAuthorityState` with exactly one active owner root and one state slot for each ordinary delegated role:

```text
OwnerAuthorityState
├── active_root
├── device_signing: DelegatedRoleState
├── administrative: DelegatedRoleState
└── recovery: DelegatedRoleState

DelegatedRoleState
├── accepted_epoch?
└── active_delegation?
```

The state owns currentness. Callers do not supply a minimum accepted delegation epoch to high-level security APIs.

Each role retains its monotonic last accepted epoch and, when valid under the active root, its exact active delegation. This prevents both arbitrary authority-object substitution and role-epoch rollback across root rotation.

### 1.2 Root transition

Normal root replacement uses existing `RootSuccessor` verification. State mutation happens only after the successor validates completely.

A successful root replacement:

1. makes the previous root historical, not current ordinary authority;
2. clears every active delegated-role object because it was authorized by the superseded root;
3. retains each role's accepted epoch floor;
4. requires a strictly newer delegation, signed by the new active root, before that role becomes active again.

This keeps root rotation fail-closed without resetting delegated-role monotonicity.

This slice does not design emergency root recovery.

### 1.3 Delegation transition

When no earlier role epoch is known, the first accepted delegation for a role verifies under the active root and may begin at any valid epoch.

When an earlier role epoch is known, a replacement must:

1. match the state owner;
2. use the expected delegated role;
3. verify under the active root;
4. carry an epoch strictly greater than the retained role floor;
5. verify completely before state mutation.

Equal/lower, wrong-owner, wrong-role, wrong-root, bad-signature, or ambiguous replacements fail without changing state.

A missing active delegation fails closed even if a historical accepted epoch is retained.

## 2. Authority-bearing API boundary

High-level authority consumers migrate to `&OwnerAuthorityState`.

Expected migration boundary:

- `DeviceCredential` issue/verify/rotate paths select current Device Signing authority from state;
- pairing inviter/joiner credential and trust-establishment paths use state;
- `PairingTrustTransition` and accepted credential rotation use state;
- `OwnerApprovalEvidence` uses the current Administrative authority;
- delegated revocation selects the current accepted delegation for its signed role;
- root revocation uses the active root from state;
- session authentication uses state and no longer carries a raw issuer delegation per handshake side.

`AuthorityDelegation::issue`, raw cryptographic import constructors, deterministic vector constructors, and low-level signature validation may continue to take explicit authority objects where currentness is not being established.

The boundary rule is: **cryptographic validity may be checked with explicit objects; production current authority is obtained from authoritative local state.**

## 3. Session currentness after authority replacement

Session authentication records enough local authority identity to revalidate the authentication basis:

- active root epoch/key identity used for authentication;
- current Device Signing delegation epoch/key identity used for credential verification.

When local state accepts a new root or Device Signing delegation, an ordinary session authenticated under the superseded authority becomes stale.

The high-level session/runtime path must then:

1. stop accepting new ordinary work;
2. cancel session-scoped control/request state;
3. revoke/cancel active `AuthorizedOperation` authority;
4. stop accepting new data streams;
5. close the logical session/transport as soon as practical;
6. require fresh authentication on reconnect.

Root replacement also leaves Device Signing inactive until a strictly newer Device Signing delegation under the new root is accepted, so fresh ordinary session authentication fails closed during that interval.

Administrative or Recovery rotation alone does not invalidate an ordinary device session.

The implementation should reuse the existing fail-closed trust/credential-currentness lifecycle instead of inventing a parallel session manager.

## 4. Request replay and retry model

### 4.1 Full-session wire replay/order boundary

The control channel's authenticated `(SessionId, message_seq)` state is authoritative for full-session exact-envelope replay/order integrity.

Phase 1 keeps one ordered reliable control channel per direction. Sequence values advance exactly by one; duplicate/lower and gaps fail according to the existing protocol/session rules.

An exact old envelope with its old sequence is rejected.

An authenticated malicious peer may still intentionally create a **new** envelope using the next valid sequence while repeating an earlier operation body or `RequestId`. That is a new request attempt, not wire replay, and it must pass current trust, capability, policy, operation, and bounded duplicate-state checks.

This distinction matters because an authorized malicious peer could choose a fresh `RequestId` for the same operation anyway. Permanent `RequestId` retention is therefore not the security boundary for semantic single-use behavior.

### 4.2 Bounded request history

`RequestId` remains a random session-scoped correlation/duplicate/retry identifier.

`ControlDispatcher` keeps bounded state for:

- in-flight inbound requests;
- pending outbound requests;
- recent completed/cancelled inbound IDs;
- future capability-authorized idempotent result cache entries.

Active/in-flight state is never evicted merely to preserve completed history.

Recent duplicate IDs fail closed. Once an ancient completed ID legitimately ages out of the bounded history, a later use of that identifier is evaluated as a new request attempt and must pass current authorization and operation semantics.

### 4.3 Retry provenance

Peer `RetryClass::Idempotent` is descriptive input, not authorization.

A duplicate may be re-executed or served from a recorded result only when local capability metadata explicitly declares matching idempotent semantics. Until that capability-local path exists, retained duplicates are rejected.

`NonRetryable` means the generic sender/protocol layer must not automatically retry the request. It does not create a permanent receiver-side exactly-once guarantee.

Capabilities requiring single-use, bounded-use, or stronger duplicate-effect prevention must enforce it through locally authoritative capability/`AuthorizedOperation` state.

No generic durable exactly-once behavior is introduced.

## 5. Stream/operation local-currentness hardening

The existing policy architecture already says trust and policy revisions are locally authoritative. The implementation API should make that hard to misuse.

High-level stream admission/runtime APIs should derive current values from local state rather than accept arbitrary `u64` values from callers.

Preferred boundary:

```text
stream admission
  ├── active LogicalSession
  ├── local TrustRecord/current trust source
  ├── local PolicyState/current policy source
  └── DataStreamOpen
          ↓
  derive trust + policy revisions locally
          ↓
  AuthorizedOperation validation
```

The low-level `AuthorizedOperation` validator may continue to compare explicit revision values for focused tests/internal use, but platform-facing/high-level runtime code must not ask a less-trusted caller to declare what revision is current.

This is implementation hardening of the existing Policy/Authorization specification, not a new independent ADR.

## 6. System events

No new generic system-event authority mechanism is added in this slice.

Current generic system events remain opaque protocol data. Before a real system-event family can trigger privileged, policy-sensitive, or platform authority, that family must define explicit local authorization/subscription semantics and fail-closed handling.

Do not add speculative global event registries merely to anticipate future families.

## 7. Persistence boundary

Phase 1 authority state remains in-memory.

Before platform production state relies on authority currentness across restart, local persistence must provide atomic durable storage for at least:

- active owner root;
- each delegated role's accepted epoch floor;
- each delegated role's active delegation when present;
- associated schema/version metadata needed to revalidate on load.

Load must revalidate cryptographic/structural relationships and fail closed on missing, torn, rollback-ambiguous, or conflicting authority state.

CRDT synchronization is not authoritative for owner root, delegated-role currentness, revocation, recovery, or administrative security state.

## 8. Error handling

Prefer existing typed identity/policy/core errors where they remain semantically accurate. Add narrowly scoped errors only when the caller needs to distinguish a real state transition failure.

No stringly typed security states.

State mutation follows validate-then-commit. Failed validation never partially advances authority currentness.

Authority/replay ambiguity fails closed.

## 9. Regression-first verification obligations

### Authority state

- valid first delegation is accepted;
- a first observed valid delegation may start above epoch zero;
- wrong owner/root/role/signature is rejected;
- strictly higher replacement succeeds;
- equal/lower/alternate same-epoch replacement is rejected;
- valid root successor advances active root;
- root successor clears active delegated-role objects but retains their accepted epoch floors;
- new-root delegation must strictly advance the retained role epoch;
- old root cannot authorize new high-level root operations after successor acceptance;
- old Device Signing delegation cannot issue/verify fresh high-level credential/session authority after replacement;
- old Administrative delegation cannot produce accepted owner approval after replacement;
- stale delegated revocation fails;
- stale pairing Device Signing authority fails.

### Session lifecycle

- session authenticated under Device Signing `N` closes/fails currentness after local `N+1` acceptance;
- session authenticated under root `N` closes/fails currentness after root `N+1` acceptance;
- fresh session authentication fails while root has advanced but no newer Device Signing delegation is active;
- Administrative-only rotation does not close ordinary session;
- Recovery-only rotation does not close ordinary session;
- authority-currentness failure cancels session-scoped control/operation/stream authority.

### Replay/request state

- active duplicate request ID is rejected;
- recent completed/cancelled duplicate is rejected;
- completed history remains bounded;
- active request state is not evicted by history pressure;
- peer-declared idempotence cannot authorize duplicate execution;
- an aged-out ID is treated as a new request and still passes current policy/operation checks;
- capability/operation single-use semantics do not rely on permanent `RequestId` retention;
- message sequence replay/gap protections remain unchanged;
- reconnect starts fresh request/sequence state and does not inherit exactly-once assumptions.

### Stream local state

- stale trust revision invalidates stream operation when local `TrustRecord` has advanced;
- stale policy revision invalidates stream operation when local `PolicyState` has advanced;
- high-level stream admission no longer accepts caller-selected current revision numbers;
- revoked trust cannot be hidden behind stale numeric inputs.

### Compatibility

- existing cryptographic canonical transcripts remain unchanged;
- identity/policy/protocol golden vectors remain unchanged unless a test is specifically about new in-memory state;
- protobuf schema/wire bytes remain unchanged;
- Quinn channel binding and transport contract remain unchanged.

## 10. Verification gate

Before the foundation remediation is considered complete:

1. focused RED evidence is captured for each defect/unsafe API boundary;
2. minimal GREEN implementation is verified locally/on exact branch head;
3. `cargo fmt --check` passes;
4. `cargo check --workspace --all-targets --all-features` passes;
5. `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes;
6. `cargo test --workspace --all-features` passes;
7. dependency audit passes with only explicitly documented accepted warnings;
8. fuzz smoke passes for affected parser/security surfaces;
9. `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md` is reconciled with actual code;
10. `docs/development/CURRENT.md` records exact final commits/workflow runs and remaining deferred risks;
11. M9 Task 4 remains blocked until the final remediation checkpoint is merged and verified on `main`.

## 11. Non-goals

This design does not:

- choose the M9 remote networking architecture;
- implement ADR-0009;
- introduce a database or production local persistence subsystem;
- define emergency root recovery;
- introduce CRDT security-authority merge semantics;
- change protobuf schemas or cryptographic transcript bytes;
- add 0-RTT/session resumption authority;
- add generic durable exactly-once requests;
- build platform adapters, privileged helpers, UI, or plugin APIs.
