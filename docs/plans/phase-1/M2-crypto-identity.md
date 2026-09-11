# M2 — Crypto + Identity

**Phase:** Phase 1 — Core Simulator  
**Status:** Complete  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.1  
**Primary specifications:** `docs/architecture/IDENTITY-AND-KEYS.md`, `docs/protocol/PROTOCOL-V1.md`  
**Relevant ADRs:** ADR-0002, ADR-0006

## Objective

Implement the narrow cryptographic and identity foundation required by later trust, protocol, pairing, and session milestones without leaking cryptography-library types into Cross-Lab domain APIs or introducing transport/platform behavior.

## Delivered

### Cryptographic foundation

`crosslab-crypto` now provides:

- canonical transcript v1 construction with exact framing and ordered field tags;
- BLAKE3-256 helpers for transcript and signed-object digests;
- Ed25519 signing/verifying wrappers for profile v1;
- strict Ed25519 verification and weak-public-key rejection;
- OS-backed secure random byte generation;
- redacted signing-key debug output and zeroization support for sensitive transient material.

The crate remains implementation-neutral at its public Cross-Lab boundary and contains no identity, policy, protocol, session, networking, or platform semantics.

### Identity foundation

`crosslab-identity` now provides:

- typed 256-bit `OwnerId`, `DeviceId`, and `KeyId` values;
- profile-v1 `KeyId` derivation;
- `OwnerRootRecord`;
- explicit `AuthorityRole` values;
- signed `AuthorityDelegation` issue/verification for delegable roles;
- rejection of `AuthorityRole::OwnerRoot` through ordinary delegation, keeping root replacement on the `RootSuccessor` continuity path;
- signed `DeviceCredential` issue/verification;
- credential epochs and device-key rotation preserving `DeviceId`;
- normal owner-root successor verification requiring current- and next-root continuity signatures;
- typed identity failures required by the current implementation surface.

Credential-bound device public keys support later session proof-of-possession verification without introducing session semantics into the identity crate.

## Security Tests

Tests cover:

- exact canonical transcript v1 layout and domain separation;
- invalid/non-increasing canonical field tags;
- Ed25519 signature success and modified-digest rejection;
- weak Ed25519 public-key rejection;
- secret-safe debug formatting;
- signed-object digest binding;
- stable ID and `KeyId` behavior;
- owner-root/delegation verification;
- rejection of Owner Root creation through ordinary authority delegation;
- wrong root, wrong owner, wrong issuer key, and wrong issuer role;
- stale authority and credential epochs;
- recovery authority rejected for ordinary device credential issuance;
- device-key rotation preserving the stable `DeviceId`;
- credential-bound proof of possession;
- normal root successor continuity and rejection of the wrong current root key.

## Regression Vectors

Fixed synthetic keys are used to freeze authority-delegation and device-credential transcript digests and Ed25519 signatures.

These fixtures are deterministic regression vectors generated from the current conforming implementation. They protect Cross-Lab against accidental signing-contract changes, but they are not yet claimed as independently cross-implemented interoperability vectors. Cross-implementation vector validation can be added when another protocol implementation or independent fixture generator exists.

## Dependencies

M2 introduces only the dependencies required by the accepted v1 profile:

- `blake3` 1.8.7;
- `ed25519-dalek` 3.0.0;
- `getrandom` 0.4.3;
- `zeroize` 1.9.0.

Versions are captured in `Cargo.lock`. No networking, serialization, persistence, UI, platform, plugin, or privileged-service dependency was introduced.

## Verification

Implementation head `b34082156fe4e69aa8f26c1ba2fc0485ca66218a` passed GitHub Actions run `34553938717` with Rust 1.98.1:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

The final documentation checkpoint is verified separately before integration.

## Handoff

Proceed to **M3 — Trust + Policy** after this milestone is integrated into `main`.

M3 owns trust/revocation records, validated capability/policy domain types, the deterministic policy evaluator, synthetic approval evidence, and operation-scoped authorization lifecycle. Pairing orchestration, protobuf/wire framing, logical sessions, real networking, persistence, and platform APIs remain later milestones.
