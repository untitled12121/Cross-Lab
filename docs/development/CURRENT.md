# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M3 — Trust + Policy: complete and integrated into `main`.**

The next implementation milestone is **M4 — Protocol**.

## Branch State

- `main` — canonical integrated branch; includes verified M1, M2, and M3.
- `trust-policy` — completed M3 branch with no unique work after PR #8 integration; may be retired.
- `foundation` — completed M2 branch with no required unique M3 work.
- `planning` — planning/documentation branch; no active implementation belongs here.

Create a fresh `protocol` branch from current `main` before M4 implementation. Do not continue M4 work on `trust-policy`.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator and milestone specification.
- `docs/architecture/IDENTITY-AND-KEYS.md` — implemented M2 identity contract.
- `docs/architecture/PAIRING-TRUST-REVOCATION.md` — implemented M3 trust/revocation contract.
- `docs/architecture/POLICY-AUTHORIZATION.md` — implemented M3 policy/operation contract.
- `docs/protocol/PROTOCOL-V1.md` — primary M4 protocol specification.
- ADR-0002 and ADR-0006 — accepted identity cryptographic profile and focused crypto boundary.

## M3 Integrated

M3 establishes the platform-independent trust and authorization domain required by the Core Simulator:

- typed trust state, trust revisions, credential epochs, and transition IDs;
- signed owner-root and delegated ordinary revocation transitions;
- fail-closed verification of owner/device binding, issuer authority, signatures, credential epochs, and revisions;
- no public unsigned mutation path for trust revocation;
- canonical typed capability IDs, operation names, versions, version ranges, and runtime availability;
- deterministic exact-device policy rules with `Allow`, `Deny`, and `Ask` decisions;
- scoped synthetic local approval evidence;
- decision metadata including matched rule, constraints, reason, and policy revision;
- cryptographically random operation IDs created only from an allow grant;
- operation binding to source, destination, logical session, capability, version, and operation;
- trust/policy revision and constraint snapshots;
- bounded operation lifetime and terminal cancellation/expiry/revocation/consumption behavior;
- fixed v1 trust-revocation digest/signature regression vector and negative security coverage.

No pairing orchestration, protobuf/wire framing, logical-session runtime, real networking, persistence, UI, platform adapter, plugin, or privileged-service implementation was introduced.

PR #8 merged M3 into `main` as `0d83b43d8e15f849344987bda62022331494caac`.

## Verification

Implementation head `2308d7fdaaa8e0a67abac5fe70e42c6abc7d9b67` passed GitHub Actions run `34560029752` on Rust 1.98.1. The final M3 documentation head `2543d5627f86d508dcd3b2363e3831b35be09fe8` passed pull-request run `34560264063` against the then-current `main`. Both runs completed the configured verification gates successfully:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

GitHub recorded PR #8 as merged without conflict. The merge commit contains the verified `trust-policy` tree plus this integration history.

## Exact Next Task

1. inspect current `main`, recent commits, open PRs, branches, and this handoff;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`, `docs/protocol/PROTOCOL-V1.md`, and the protocol/M4 sections of `docs/architecture/CORE-SIMULATOR.md`;
3. create a fresh `protocol` implementation branch from current `main`;
4. implement M4 test-first, beginning with protocol version/range negotiation and bounded framing/domain-conversion foundations;
5. continue with protobuf v1 messages, strict conversions, control/data-stream headers, compatibility/error handling, parser tests, golden vectors, and the protocol fuzz targets required by the accepted specifications.

Do not introduce pairing orchestration, authenticated logical sessions, real networking, persistence, UI, platform adapters, plugins, or privileged services during M4.

## Resume Procedure

Before continuing in a new session:

1. inspect repository/branch/PR state and recent commits;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read this file;
4. read `docs/protocol/PROTOCOL-V1.md` and the M4/protocol sections of `docs/architecture/CORE-SIMULATOR.md`;
5. reconcile the documentation with actual code and CI state;
6. continue from the exact next task above.
