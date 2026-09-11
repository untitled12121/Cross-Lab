# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M3 — Trust + Policy: complete and verified on `trust-policy`.**

PR #8 is the integration vehicle into `main`. After that integration, the next implementation milestone is **M4 — Protocol**.

## Branch State

- `main` — canonical integrated branch; PR #8 targets it with the completed M3 implementation.
- `trust-policy` — completed M3 branch; keep until PR #8 is integrated.
- `foundation` — completed M2 branch with no required unique M3 work.
- `planning` — planning/documentation branch; no active implementation belongs here.

Do not start M4 implementation on `trust-policy`.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator and milestone specification.
- `docs/architecture/IDENTITY-AND-KEYS.md` — implemented M2 identity contract.
- `docs/architecture/PAIRING-TRUST-REVOCATION.md` — implemented M3 trust/revocation contract.
- `docs/architecture/POLICY-AUTHORIZATION.md` — implemented M3 policy/operation contract.
- `docs/protocol/PROTOCOL-V1.md` — primary M4 protocol specification.
- ADR-0002 and ADR-0006 — accepted identity cryptographic profile and focused crypto boundary.

## M3 Delivered

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

## Verification

Implementation head `2308d7fdaaa8e0a67abac5fe70e42c6abc7d9b67` passed GitHub Actions run `34560029752` on Rust 1.98.1. The run verified the PR merge result against the then-current `main` and completed all configured gates successfully:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

The documentation checkpoint containing this handoff must also pass the same PR workflow before PR #8 is merged. GitHub PR/CI state is authoritative for that final integration check.

## Exact Next Task

After PR #8 is integrated into `main`:

1. inspect `main`, recent commits, open PRs, branches, and this handoff;
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
