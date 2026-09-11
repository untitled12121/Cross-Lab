# M3 — Trust + Policy

**Phase:** Phase 1 — Core Simulator  
**Status:** Ready after M2 integration  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.1  
**Primary specifications:** `docs/architecture/PAIRING-TRUST-REVOCATION.md`, `docs/architecture/POLICY-AUTHORIZATION.md`  
**Supporting specification:** `docs/architecture/CORE-SIMULATOR.md`

## Objective

Implement the deterministic trust and authorization domain required by the Core Simulator. M3 establishes explicit trust/revocation state, validated capability/policy types, a pure fail-closed policy evaluator, synthetic local approval evidence, and bounded operation-scoped authority.

M3 must not introduce pairing orchestration, wire serialization, sessions, real networking, persistence, UI prompts, biometrics, platform adapters, or privileged operations.

## Scope

### Trust state

Implement typed trust records with the Phase 1 states:

```text
Pending
Trusted
Revoked
```

A trust record carries owner/device identity, accepted credential epoch, trust revision, and last transition identifier. Ordinary authorization requires `Trusted`; `Pending` and `Revoked` must fail closed.

Implement the trust/revision behavior required by the simulator, including:

- initial trusted record creation from an already validated credential/pairing result;
- accepted credential-epoch advancement for the same `DeviceId`;
- monotonic trust revisions;
- terminal ordinary revocation semantics for Phase 1;
- hooks/results needed to invalidate operation authority after trust changes.

Signed pairing/revocation orchestration remains bounded to the ownership defined by the trust specification and must use the existing crypto boundary if introduced in this milestone.

### Capability domain types

Implement strongly typed validated values for:

- `CapabilityId` using the exact lowercase ASCII dotted grammar and 128-byte maximum;
- `OperationName` using one canonical segment and 64-byte maximum;
- `CapabilityVersion` and compatible local capability inputs required by policy;
- runtime availability and the minimum typed security/context values required by M3 tests.

Do not accept unchecked strings as authorization identifiers.

### Policy state and evaluator

Implement deterministic exact-device policy state keyed by:

```text
(source_device_id, capability_id, operation_name)
```

Implement typed:

- `PolicyRule`;
- `RuleEffect` (`Allow`, `Deny`, `Ask`);
- Phase 1 constraints actually required by simulator tests;
- Phase 1 obligations actually required by simulator tests;
- locally verified synthetic approval evidence;
- `AuthorizationContext`;
- `PolicyDecision` and typed `DecisionReason`.

Evaluation follows the specification order and is pure: no UI, networking, persistence, key generation, or privileged/platform calls occur inside policy evaluation.

No matching rule is `Deny`.

### Authorized operations

Only a final `Allow` may create an `AuthorizedOperation`.

Implement:

- cryptographically random 256-bit `OperationId`;
- source/destination/session/capability/version/operation binding;
- trust and policy revision snapshots;
- bounded lifetime/expiry state supplied by deterministic simulator context;
- typed use policy needed by current tests;
- terminal `Active -> Cancelled | Expired | Revoked | Consumed` transitions;
- validation that possession of an `OperationId` alone grants no authority;
- invalidation after relevant policy/trust revision change.

## Dependency and Boundary Rules

- `crosslab-policy` remains platform-independent and deterministic.
- Reuse `crosslab-identity` domain types rather than duplicating identity state.
- Use `crosslab-crypto` only for generic cryptographic mechanics where the approved trust domain requires them; keep trust/policy semantics in `crosslab-policy`.
- Do not add protobuf, Quinn, database, GPUI, mobile, plugin, or privileged-service dependencies.
- Keep public policy types strongly typed and avoid extension-by-arbitrary-string escape hatches.

## Required Tests

M3 must cover at least:

- valid and invalid `CapabilityId`/`OperationName` grammar and limits;
- no rule -> deny;
- exact allow rule + satisfied constraints -> allow;
- explicit deny -> deny;
- untrusted/revoked source -> deny;
- unsupported/runtime-unavailable capability -> deny;
- incompatible capability version -> deny;
- false hard constraint -> deny;
- missing interactive approval -> ask;
- locally verified approval -> allow after reevaluation;
- spoofed peer approval cannot satisfy a local obligation;
- `OperationId` created only after allow;
- wrong source/session/capability/operation with a valid `OperationId` -> deny;
- expired/cancelled/revoked/consumed operation -> deny;
- policy revision invalidates active operation;
- trust revision/revocation invalidates active operation;
- credential-epoch advancement preserves trusted device identity while rejecting stale epochs.

Tests should be deterministic except where testing secure-random shape/uniqueness properties.

## Verification

Run from the repository root:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Acceptance Criteria

M3 is complete when:

- trust/revocation state and revision semantics required by Phase 1 are implemented and tested;
- capability and operation identifiers are validated typed values;
- policy evaluation is deterministic, pure, exact-rule based, and fail-closed;
- interactive obligations can only be satisfied by typed locally verified evidence;
- operation authority is created only after `Allow`, is strongly bound to context, and becomes unusable after terminal/revision-invalidating changes;
- no platform/network/persistence/pairing/session behavior leaks into the policy domain;
- the full workspace verification baseline is green;
- `docs/development/CURRENT.md` records the verified M3 checkpoint and exact M4 task.

## Handoff

After M3, proceed to **M4 — Protocol** for protobuf schemas/codecs, compatibility negotiation, bounded framing, envelopes/request/event/cancel/error/data-stream headers, protocol vectors, and parser/compatibility tests.
