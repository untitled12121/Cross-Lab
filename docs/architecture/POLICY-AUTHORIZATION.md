# Cross-Lab Policy and Authorization

**Status:** Phase 0 specification  
**Milestone:** P0.5 — Policy / Authorization Model  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0

## 1. Purpose

This specification defines capability identifiers, capability compatibility inputs required by policy, the Phase 1 permission-rule model, authorization context, interactive obligations, and the lifecycle of operation-scoped authority.

The central rule is:

```text
Capability = can
Policy     = may
```

Neither a valid device credential nor `TrustState::Trusted` grants capability authority by itself.

## 2. Capability identifiers

### 2.1 `CapabilityId`

`CapabilityId` is a validated canonical identifier, not an arbitrary string.

Canonical textual grammar for v1:

```text
segment *("." segment)
segment = lowercase ASCII letter *(lowercase ASCII letter / digit / "-")
```

Additional rules:

- at least two segments;
- maximum canonical length: 128 bytes;
- no empty segment;
- no leading/trailing `-` within a segment;
- comparison is byte-exact after validation; no Unicode case folding/normalization;
- capability IDs are protocol/domain identifiers, not display labels.

Examples:

```text
clipboard.read
clipboard.write
files.transfer
screen.capture
remote.shell
biometric.approve
```

Invalid examples include uppercase, whitespace, Unicode lookalikes, empty segments, and identifiers exceeding the protocol limit.

### 2.2 `OperationName`

A capability may expose one or more validated operation names. `OperationName` uses one canonical segment under the same ASCII segment rules with a maximum of 64 bytes.

The effective protected operation is the pair:

```text
(CapabilityId, OperationName)
```

Examples:

```text
(files.transfer, send)
(files.transfer, receive)
(screen.capture, start)
(screen.capture, stop)
```

The type is validated/strongly wrapped in code; policy does not accept unchecked `String` values as operation authority.

## 3. Capability versions

Phase 1 represents a capability version as:

```text
CapabilityVersion
  major: u16
  minor: u16
```

Compatibility rules are finalized in P0.6. Policy receives only a negotiated compatible version, never raw peer preference as if it were locally accepted support.

A runtime capability record includes:

```text
LocalCapability
  capability_id
  supported_version_range
  runtime_available
  required_integration_level
  local_limits
```

`runtime_available = false` or incompatible version causes authorization denial/unsupported result before an operation is created.

## 4. Capability registry rule

The local capability registry is authoritative for what the local device can currently perform. Remote advertisements describe remote support but cannot cause a local unsupported capability to appear.

Runtime permission loss, hardware removal, platform policy, or integration-level changes may make a previously advertised capability unavailable. Active operation behavior must then follow capability-specific cancellation/failure semantics and cannot assume availability forever.

## 5. Phase 1 policy state

Phase 1 intentionally does not implement a general policy programming language.

Policy state is a deterministic collection of exact device-scoped rules:

```text
PolicyState
  revision: u64
  rules: set keyed uniquely by
         (source_device_id, capability_id, operation_name)
```

A duplicate key is invalid configuration rather than an implicit precedence contest.

Future user/group/family/wildcard policy requires an explicit later design; Phase 1 does not need it.

## 6. Policy rule

```text
PolicyRule
  rule_id: 256-bit random
  source_device_id: DeviceId
  capability_id: CapabilityId
  operation: OperationName
  effect: RuleEffect
  constraints: set<Constraint>
  obligations: set<Obligation>
```

### 6.1 Rule effect

```text
RuleEffect
  Allow
  Deny
  Ask
```

`Deny` is explicit denial.

`Ask` requires an interactive/local approval workflow before a bounded authorization may be created.

`Allow` still requires all hard constraints and mandatory obligations to be satisfied.

No matching rule = `Deny`.

## 7. Constraints

Constraints are non-interactive conditions that must already be true:

```text
Constraint
  LocalOnly
  TrustedNetworkOnly
  RemoteAllowed
  RecoveryOnly
  RequiredRiskState(...)
  RequiredLifecycleState(...)
  ExpiresAt(...)
```

Phase 1 implements only constraints needed by simulator tests. The enum/type may reserve no stringly-typed extension escape hatch.

A false hard constraint produces `Deny`, not `Ask`.

`RemoteAllowed` is not an authorization grant by itself; it permits a rule to remain eligible in remote network context.

## 8. Obligations

Obligations require locally verified evidence before an operation becomes authorized:

```text
Obligation
  OwnerConfirmation
  BiometricApproval
  SecondApproval
```

Presence of a peer-provided boolean such as `biometric=true` does not satisfy an obligation. Approval evidence must originate from the local trusted approval mechanism defined for that obligation.

Phase 1 may model approval evidence deterministically in the simulator; it does not implement platform biometrics.

## 9. Authorization context

The policy evaluator receives a fully typed context assembled by trusted local components:

```text
AuthorizationContext
  source_device_id
  destination_device_id
  session_id
  capability_id
  negotiated_capability_version
  operation
  trust_state
  trust_revision
  risk_state
  lifecycle_state
  recovery_state
  network_classification
  route_security_class
  requested_privilege
  verified_approvals
  local_time_context
  policy_revision
```

### 9.1 Provenance

Context fields have an explicit provenance:

- peer identity comes from Cross-Lab authentication;
- trust/revision comes from local trust state;
- capability compatibility comes from negotiated + local capability state;
- network/route class comes from local transport/orchestrator evidence;
- platform/risk/lifecycle state comes from local state;
- approvals come from local trusted approval mechanisms;
- policy revision comes from local policy state.

Remote peer claims may be input data but cannot replace locally authoritative fields.

## 10. Evaluation order

The evaluator follows this logical order:

1. authenticated source identity exists and matches the session;
2. source trust is currently `Trusted` for ordinary operations;
3. destination/runtime capability exists and is available;
4. capability version is compatible/negotiated;
5. security/route prerequisites are eligible;
6. find exact rule for `(source, capability, operation)`;
7. no rule -> `Deny`;
8. `Deny` rule -> `Deny`;
9. evaluate hard constraints; any failure -> `Deny`;
10. determine unsatisfied interactive obligations;
11. `Ask` rule or missing required approval -> `Ask` with required obligations;
12. otherwise -> `Allow` with the satisfied constraints/obligations recorded.

The evaluator is pure/deterministic for a supplied context and policy state. It does not perform UI prompts, network requests, key operations, persistence, or privileged OS actions.

## 11. `PolicyDecision`

```text
PolicyDecision
  effect: Allow | Deny | Ask
  matched_rule_id: optional RuleId
  constraints
  required_obligations
  reason: typed DecisionReason
  policy_revision
```

`DecisionReason` is a typed enum/code suitable for tests/audit/UI mapping. Human-readable text is not part of authorization semantics.

Required reasons include at least:

```text
NoMatchingRule
ExplicitDeny
UntrustedPeer
RevokedPeer
UnsupportedCapability
IncompatibleCapabilityVersion
RuntimeUnavailable
ConstraintFailed
ApprovalRequired
SecurityRouteIneligible
Allowed
```

## 12. Approval workflow

An `Ask` result does not create operation authority.

The caller may obtain locally verified approval evidence and reevaluate policy with the updated context. Approval evidence must be:

- scoped to the intended source/destination/capability/operation or approval request;
- bounded in lifetime;
- non-reusable outside its defined scope;
- invalidated when the underlying request/session is cancelled.

Exact biometric/platform approval formats are platform milestones. The simulator uses synthetic typed approval evidence.

## 13. Authorized operation

Only a final `Allow` result may create an `AuthorizedOperation`:

```text
AuthorizedOperation
  operation_id: OperationId
  source_device_id
  destination_device_id
  session_id
  capability_id
  capability_version
  operation
  trust_revision
  policy_revision
  constraints_snapshot
  created_at
  expires_at
  use_policy
  state: Active | Cancelled | Expired | Revoked | Consumed
```

### 13.1 `OperationId`

`OperationId` is a cryptographically random 256-bit identifier.

It is a correlation handle, not a bearer capability. Knowledge of the ID alone is insufficient: the stream/request must also match the authenticated source, destination, logical session, capability/operation, direction, lifetime, and use policy recorded in the operation.

## 14. Operation lifetime

Every protected operation has a bounded lifetime, even if a future UI makes long-running authorization convenient.

Phase 1 uses explicit local expiry/deadline state. Local monotonic time should be used for in-process deadlines where possible; protocol timestamps are not trusted as a replacement for local authorization state.

States are terminal except where a capability-specific later design explicitly permits a new operation:

```text
Active -> Cancelled
Active -> Expired
Active -> Revoked
Active -> Consumed
```

An inactive operation cannot authorize a new data stream/action.

## 15. Policy and trust revision binding

`AuthorizedOperation` records the trust and policy revisions used to authorize it.

For Phase 1, use the conservative rule:

- a trust revision change for the source device invalidates its active/pending authorized operations;
- any local policy revision change invalidates active/pending authorized operations on that destination and requires reauthorization for further protected work.

This is intentionally simple and safe for the simulator. A future optimization may track more granular rule dependencies through an ADR/specification, but must not allow stale authority after a security-relevant policy change.

## 16. Revocation

When a trusted source becomes revoked:

- policy evaluation returns denial for ordinary operations;
- all its `AuthorizedOperation` records become `Revoked`;
- new data-stream opens referencing those operations fail;
- active ordinary session behavior follows P0.7 and is terminated as required by P0.4.

A newer credential epoch does not bypass revoked trust.

## 17. Control/data-plane binding

A protected data-stream open supplies an `OperationId` and its stream purpose/direction. The receiver retrieves local operation state and validates:

- state is `Active`;
- source/destination/session match;
- capability/version/operation match;
- stream purpose/direction is permitted;
- lifetime valid;
- use count/policy permits another stream;
- trust/policy revision still valid;
- current revocation state permits continuation.

Failure is explicit and fail-closed.

## 18. Use policy

An operation defines one of the bounded use modes needed by its capability:

```text
UsePolicy
  SingleAction
  SingleStream
  MultiStream { max_streams }
  LongRunningSessionScoped
```

Any count is explicitly bounded. `LongRunningSessionScoped` still expires/cancels/revokes with the parent session and does not create cross-session ambient authority.

Phase 1 uses only modes required by simulator scenarios.

## 19. Cancellation

Cancellation is explicit and idempotent. Cancelling an operation:

- prevents new work under its `OperationId`;
- signals active tasks/streams to stop according to capability semantics;
- releases bounded resources;
- does not change peer trust;
- produces an audit/control result where required.

A cancelled operation ID cannot be reactivated.

## 20. Policy persistence

Persistent policy storage is outside Phase 1. The simulator uses in-memory typed policy state.

When persistence is introduced, storage format/migrations must preserve policy revisions and fail safely on corrupt/unknown security state. SQLite or another database remains unselected until a feature requires it.

## 21. Phase 1 required tests

The policy/core implementation must cover at least:

- no rule -> deny;
- exact allow rule + satisfied constraints -> allow;
- explicit deny -> deny;
- trusted peer without capability permission -> deny;
- unsupported/runtime-unavailable capability -> deny;
- incompatible capability version -> deny;
- false hard constraint -> deny;
- missing interactive approval -> ask;
- verified approval -> allow after reevaluation;
- spoofed remote approval/context cannot satisfy local evidence;
- random `OperationId` created only after allow;
- wrong source/session/capability with valid `OperationId` -> deny;
- expired/cancelled/revoked/consumed operation -> deny;
- policy revision invalidates active operation;
- trust revision/revocation invalidates active operation.

## 22. Security traceability

This specification addresses `TM-006`, `TM-007`, `TM-008`, `TM-009`, `TM-010`, `TM-014`, `TM-017`, `TM-018`, and `TM-022`.

A future richer rule language, wildcard/group authorization, bearer-capability semantics, or granular stale-operation policy would materially affect the security model and requires explicit design/ADR review.
