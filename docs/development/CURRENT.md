# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M2 — Crypto + Identity: complete and integrated into `main`.**

**M3 — Trust + Policy: next implementation milestone; design approval is the immediate gate before code changes.**

## Branch State

- `main` — canonical integrated branch; includes verified M1 and M2.
- `foundation` — completed M2 branch; contains no unique work and may be retired.
- `planning` — planning/documentation branch; contains no unique work and should remain synchronized when idle.
- next implementation branch after M3 design approval: `trust-policy`.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator specification.
- `docs/architecture/IDENTITY-AND-KEYS.md` — implemented M2 identity contract.
- `docs/architecture/PAIRING-TRUST-REVOCATION.md` — M3 trust/revocation contract.
- `docs/architecture/POLICY-AUTHORIZATION.md` — M3 policy/operation contract.
- `docs/protocol/PROTOCOL-V1.md` — canonical transcript and signing rules.
- ADR-0002 and ADR-0006 — accepted identity cryptographic profile and focused crypto boundary.

## M2 Integrated

M2 delivered:

- canonical transcript v1 encoding and domain-separated BLAKE3 transcript/signed-object digests;
- Ed25519 wrappers with weak-public-key rejection and strict signature verification;
- OS-backed secure randomness and secret-safe handling boundaries;
- typed `OwnerId`, `DeviceId`, and `KeyId`;
- owner root records, delegated authority verification, and explicit rejection of Owner Root through ordinary delegation;
- signed device credentials, credential epochs, and device-key rotation;
- two-signature owner-root successor continuity;
- deterministic signing/transcript regression vectors and negative security tests;
- credential-bound device proof-of-possession verification.

PR #7 merged into `main` as `08f899f53b12f7678e8831757cd3a8907a512c06`.

## Verification

Final M2 PR head `9f50088e2ac443f61cfe53db1b618077b14607fb` passed GitHub Actions run `34554060161` on Rust 1.98.1. The pull-request workflow tested the merge result against the then-current `main`, and PR #7 merged without conflict.

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Exact Next Task

1. review and approve the M3 Trust + Policy design against `PAIRING-TRUST-REVOCATION.md`, `POLICY-AUTHORIZATION.md`, and `CORE-SIMULATOR.md`;
2. create/use the `trust-policy` branch from current `main`;
3. implement M3 test-first, beginning with typed trust state/revisions and validated `CapabilityId`/`OperationName` domain values;
4. then implement the pure fail-closed policy evaluator and bounded `AuthorizedOperation` lifecycle.

Do not introduce pairing orchestration, protobuf/wire framing, sessions, real networking, persistence, UI, platform adapters, plugins, or privileged services during M3.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, active branches, open PRs, recent commits, and repository state;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read this file;
4. read `docs/plans/phase-1/M3-trust-policy.md`;
5. read `PAIRING-TRUST-REVOCATION.md`, `POLICY-AUTHORIZATION.md`, and the M3 ownership sections of `CORE-SIMULATOR.md`;
6. reconcile documentation with actual code and verification state;
7. continue from the exact next task above.
