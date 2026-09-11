# M3 — Trust + Policy

**Phase:** Phase 1 — Core Simulator  
**Status:** Complete  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.1  
**Primary specifications:** `docs/architecture/PAIRING-TRUST-REVOCATION.md`, `docs/architecture/POLICY-AUTHORIZATION.md`  
**Supporting specification:** `docs/architecture/CORE-SIMULATOR.md`

## Objective

Implement the deterministic trust and authorization domain required by the Core Simulator without introducing pairing orchestration, wire serialization, sessions, real networking, persistence, UI, platform adapters, or privileged operations.

## Delivered

### Trust and revocation

`crosslab-policy` now provides:

- typed `TrustState` values for `Pending`, `Trusted`, and terminal ordinary `Revoked` state;
- `TrustRecord` bound to owner/device identity, accepted credential epoch, trust revision, and last transition ID;
- exact next-epoch validation for credential rotation while preserving stable device identity;
- signed `TrustTransition` revocation objects bound to owner, device, credential epoch, issuer authority, and trust revision;
- owner-root and delegated administrative/device-signing revocation verification;
- explicit rejection of Recovery authority on the ordinary revocation path;
- strict signature, issuer, owner/device, credential-epoch, and revision checks before trust mutation;
- a private trust-record revocation mutation path so callers cannot bypass signed transition verification.

### Capability domain

The policy crate provides validated strongly typed:

- `CapabilityId` using the canonical lowercase ASCII dotted grammar and 128-byte maximum;
- `OperationName` using one canonical segment and 64-byte maximum;
- `CapabilityVersion` and compatible version ranges;
- `LocalCapability` with runtime availability used by authorization decisions.

Unchecked strings are not accepted as authorization identifiers.

### Policy evaluation

The M3 evaluator provides:

- exact device/capability/operation rules;
- `Allow`, `Deny`, and `Ask` effects;
- typed constraints and obligations required by Phase 1 tests;
- scoped synthetic locally verified approval evidence;
- deterministic `AuthorizationContext` and typed `PolicyDecision` results;
- matched-rule, constraint, reason, and policy-revision metadata;
- fail-closed behavior for missing rules, untrusted/revoked peers, unsupported capabilities, incompatible versions, unavailable runtime capabilities, and failed constraints.

Policy evaluation remains pure and performs no UI, networking, persistence, key generation, platform calls, or privileged work.

### Authorized operations

Only an `Allow` decision can yield the authorization grant used to create an `AuthorizedOperation`.

M3 implements:

- cryptographically random 256-bit `OperationId` values;
- source, destination, logical-session, capability, version, and operation binding;
- trust and policy revision snapshots;
- constraint snapshots from the allowing rule;
- explicit bounded lifetime supplied by the caller/simulator context;
- `SingleAction` and `SingleStream` use policies required by current tests;
- terminal `Cancelled`, `Expired`, `Revoked`, and `Consumed` states;
- denial after binding mismatch, expiry, terminal state, trust revision change, or policy revision change.

Possession of an `OperationId` alone does not authorize work.

## Security and Regression Tests

Tests cover:

- canonical capability/operation identifier grammar and limits;
- trust record identity, credential epochs, revisions, and terminal revocation;
- owner-root and delegated signed revocation;
- recovery-role rejection on ordinary revocation;
- wrong issuer key, forged signature, stale credential epoch, and invalid trust revision rejection;
- no-rule, explicit-deny, pending/revoked, unsupported, runtime-unavailable, incompatible-version, and constraint failures;
- scoped approval requirements and wrong-scope approval rejection;
- operation creation only from an allow grant;
- complete operation binding checks;
- expiry, cancellation, explicit revocation, consumption, and revision invalidation;
- operation constraint snapshot propagation.

A fixed synthetic v1 trust-revocation regression vector freezes the canonical transition digest and Ed25519 signature. As with the M2 vectors, it protects the current signing contract against accidental changes; independent cross-implementation vector validation remains a later interoperability activity.

## Dependencies and Boundaries

M3 adds only the existing internal `crosslab-crypto` dependency required for signed trust transitions. No protobuf, Quinn, database, GPUI, mobile, plugin, persistence, platform, or privileged-service dependency is introduced.

Trust/policy semantics remain owned by `crosslab-policy`; generic cryptographic mechanics remain in `crosslab-crypto`; owner and authority identities remain in `crosslab-identity`.

## Verification

Implementation head `2308d7fdaaa8e0a67abac5fe70e42c6abc7d9b67` passed GitHub Actions run `34560029752` on Rust 1.98.1:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

The final documentation checkpoint is verified separately by the PR workflow before integration.

## Handoff

After PR #8 is integrated, proceed to **M4 — Protocol**.

M4 owns protocol version/range negotiation, bounded frame codecs, protobuf wire messages and strict domain conversion, control/session envelopes, request/response/event/cancel/error forms, capability advertisements, data-stream open headers, compatibility/error codes, parser tests, golden protocol vectors, and the initial protocol fuzz targets defined by `PROTOCOL-V1.md` and `CORE-SIMULATOR.md`.

Pairing orchestration, authenticated logical sessions, real networking, persistence, UI, platform adapters, and privileged services remain outside M4.
