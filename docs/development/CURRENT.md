# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M2 — Crypto + Identity**

M1 — Repository Foundation is implemented and verified. M2 is ready to start.

## Branch State

- `main` — canonical integrated branch.
- `foundation` — active Phase 1 implementation branch.
- `planning` — planning/documentation branch; keep synchronized with integrated `main` when no planning work is pending.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- ADR-0001 through ADR-0006 — Accepted.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator specification.
- `docs/architecture/IDENTITY-AND-KEYS.md` — M2 identity contract.
- `docs/protocol/PROTOCOL-V1.md` — canonical transcript and signing rules used by M2.
- ADR-0002 and ADR-0006 — M2 cryptographic profile and crate boundary.

## M1 Completed

M1 established:

- a Rust 2024 workspace with `crosslab-crypto`, `crosslab-identity`, `crosslab-policy`, `crosslab-protocol`, `crosslab-core`, and `crosslab-sim`;
- Rust 1.98.1 via `rust-toolchain.toml` with Rustfmt and Clippy;
- the approved inward, acyclic Cross-Lab crate dependency direction;
- a committed lockfile;
- GitHub Actions checks for lockfile consistency, formatting, build/check, Clippy with warnings denied, and workspace tests;
- no third-party runtime dependency, networking, persistence, UI, platform integration, plugin runtime, privileged service, or feature behavior.

M1 implementation commit: `b6158d19f330b6e4afac8ac91a76e645ecb11652`.

## Verification

GitHub Actions run `34550761467` completed successfully for M1. The following checks all passed on Rust 1.98.1:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Exact Next Task

Begin M2 test-first from the accepted crypto/identity specifications:

1. verify current maintained dependency versions and licenses required for Ed25519, BLAKE3, secure randomness, constant-time verification, and secret handling;
2. implement the narrow `crosslab-crypto` primitives required by ADR-0006 without identity business semantics;
3. implement typed `OwnerId`, `DeviceId`, and `KeyId` plus the v1 identity/key profile in `crosslab-identity`;
4. add deterministic golden vectors and negative tests before credential/rotation behavior;
5. keep protocol serialization, networking, persistence, and platform adapters out of M2.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, `foundation`, `planning`, recent commits, and repository state;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read this file;
4. read `docs/architecture/IDENTITY-AND-KEYS.md`, relevant M2 sections of `CORE-SIMULATOR.md`, and `PROTOCOL-V1.md` canonical transcript rules;
5. read ADR-0002 and ADR-0006;
6. reconcile documentation with actual code and verification state;
7. continue from the exact next task above.
