# M1 — Repository Foundation

**Phase:** Phase 1 — Core Simulator  
**Status:** Complete  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.1  
**Primary specification:** `docs/architecture/CORE-SIMULATOR.md`  
**Relevant ADR:** ADR-0006 — focused `crosslab-crypto` crate boundary

## Objective

Establish the smallest production-quality Rust workspace required to begin the deterministic Core Simulator without introducing feature logic, platform integrations, real networking, persistence, or speculative repository structure.

## Delivered

The repository now contains:

```text
Cross-Lab/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── crates/
│   ├── crypto/       # crosslab-crypto
│   ├── identity/     # crosslab-identity
│   ├── policy/       # crosslab-policy
│   ├── protocol/     # crosslab-protocol
│   └── core/         # crosslab-core
└── apps/
    └── sim/          # crosslab-sim
```

The workspace uses Rust 2024 and Rust 1.98.1. Package manifests inherit common workspace metadata and forbid unsafe Rust at the workspace lint boundary.

The initial Cross-Lab dependency direction is encoded without circular dependencies:

```text
crosslab-crypto
      |
      v
crosslab-identity
      |
      v
crosslab-policy
      | \
      |  +------------------+
      v                     v
crosslab-protocol ------> crosslab-core
                              |
                              v
                         crosslab-sim
```

M1 intentionally introduces no production cryptography, serialization, networking, persistence, UI, platform, plugin, or privileged-service dependencies.

## Tooling and CI

`.github/workflows/ci.yml` runs on pull requests and pushes to `main` and verifies:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

The workflow uses `actions/checkout@v7` and installs the repository-pinned Rust 1.98.1 toolchain with Rustfmt and Clippy.

## Verification

M1 implementation commit:

```text
b6158d19f330b6e4afac8ac91a76e645ecb11652
```

GitHub Actions run:

```text
34550761467
```

Result: all lockfile, formatting, check, Clippy, and test steps completed successfully.

## Acceptance Criteria

- Minimal approved workspace layout: satisfied.
- All six packages compile on the selected stable toolchain: satisfied.
- Dependency direction is inward and acyclic: satisfied.
- No speculative platform/network/database/plugin structure: satisfied.
- Lockfile is consistent: satisfied.
- Formatting, linting, build/check, and tests: passed.
- CI runs the same verification baseline: satisfied.
- `docs/development/CURRENT.md` identifies M2 as the next milestone: satisfied.

## Handoff

Proceed to **M2 — Crypto + Identity** using `docs/architecture/IDENTITY-AND-KEYS.md`, the canonical transcript rules in `docs/protocol/PROTOCOL-V1.md`, ADR-0002, and ADR-0006. M2 must remain inside the approved crypto/identity boundaries and begin security behavior test-first.
