# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M2 — Crypto + Identity: implementation complete on `foundation`, pending integration into `main`.**

M3 — Trust + Policy is the next implementation milestone after M2 is integrated.

## Branch State

- `main` — canonical integrated branch; currently contains the verified M1 foundation.
- `foundation` — M2 implementation branch and PR #7 head.
- `planning` — planning/documentation branch; no unique implementation work should accumulate here.

After PR #7 is integrated, synchronize/retire `foundation` and use a short purpose branch such as `trust-policy` for M3.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator specification.
- `docs/architecture/IDENTITY-AND-KEYS.md` — M2 identity contract.
- `docs/architecture/PAIRING-TRUST-REVOCATION.md` — M3 trust/revocation contract.
- `docs/architecture/POLICY-AUTHORIZATION.md` — M3 policy/operation contract.
- `docs/protocol/PROTOCOL-V1.md` — canonical transcript and signing rules.
- ADR-0002 and ADR-0006 — accepted identity cryptographic profile and focused crypto boundary.

## M2 Completed

M2 implements:

- canonical transcript v1 encoding and domain-separated BLAKE3 transcript/signed-object digests;
- Ed25519 signing and verification behind Cross-Lab wrapper types;
- weak Ed25519 public-key rejection and strict signature verification;
- OS-backed secure random generation and redacted/zeroized secret handling;
- typed 256-bit `OwnerId`, `DeviceId`, and `KeyId` values with v1 key-fingerprint derivation;
- owner root records, typed authority roles and signed authority delegations;
- explicit rejection of Owner Root creation through ordinary `AuthorityDelegation`; root replacement is restricted to the two-signature `RootSuccessor` path;
- signed device credential issue/verification, credential epochs, and device-key rotation;
- normal owner-root successor continuity requiring both current and next root signatures;
- deterministic implementation-derived transcript/signature regression vectors;
- negative tests for role separation, wrong owner/root/issuer, stale authority/credential epochs, weak keys, altered proofs, and invalid root-successor authority;
- device-key proof-of-possession support through credential-bound public-key verification.

Wire serialization, pairing orchestration/HMAC confirmation, trust/policy state, sessions, networking, persistence, and platform adapters remain outside M2.

## Verification

M2 implementation head `b34082156fe4e69aa8f26c1ba2fc0485ca66218a` passed GitHub Actions run `34553938717` on Rust 1.98.1:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

The final documentation checkpoint must pass the same CI baseline before PR #7 is integrated.

## Exact Next Task

1. complete final PR #7 review and integrate M2 into `main` only after the documentation checkpoint remains green;
2. synchronize/retire the completed `foundation` branch;
3. begin `docs/plans/phase-1/M3-trust-policy.md` on a purpose branch such as `trust-policy`;
4. start M3 test-first with typed trust state/revisions plus validated capability and operation identifiers before implementing the pure policy evaluator and authorized-operation lifecycle.

Do not begin pairing orchestration, protobuf/wire framing, sessions, real networking, persistence, UI, platform adapters, plugins, or privileged services during M3.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, `foundation`, `planning`, open PRs, recent commits, and repository state;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read this file;
4. read `docs/plans/phase-1/M2-crypto-identity.md` and `docs/plans/phase-1/M3-trust-policy.md`;
5. read `PAIRING-TRUST-REVOCATION.md`, `POLICY-AUTHORIZATION.md`, and relevant protocol canonical-transcript sections;
6. read ADR-0002 and ADR-0006 where cryptographic/trust boundaries are involved;
7. reconcile documentation with actual code and CI state;
8. continue from the exact next task above.
