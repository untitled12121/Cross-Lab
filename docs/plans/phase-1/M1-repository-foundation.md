# M1 — Repository Foundation

**Phase:** Phase 1 — Core Simulator  
**Status:** Ready for implementation  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.1  
**Primary specification:** `docs/architecture/CORE-SIMULATOR.md`  
**Relevant ADR:** ADR-0006 — focused `crosslab-crypto` crate boundary

## Objective

Establish the smallest production-quality Rust workspace required to begin the deterministic Core Simulator without introducing feature logic, platform integrations, real networking, persistence, or speculative repository structure.

M1 proves that the approved crate boundaries compile cleanly, enforce the intended dependency direction, and have a reliable formatting/lint/test baseline before identity, policy, protocol, or simulator behavior is implemented.

## Scope

Create the initial workspace defined by Master Architecture Revision 2.1:

```text
Cross-Lab/
├── Cargo.toml
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

Only files required for a compilable, testable foundation belong in this milestone.

## Deliverables

### Workspace

- root Cargo workspace manifest;
- stable Rust toolchain configuration;
- shared workspace package metadata and lints where useful;
- deterministic resolver/profile settings appropriate for the current stable toolchain.

### Crate shells

Create minimal package shells for:

- `crosslab-crypto`;
- `crosslab-identity`;
- `crosslab-policy`;
- `crosslab-protocol`;
- `crosslab-core`;
- `crosslab-sim`.

Each crate/application must have a narrow public purpose and no placeholder business architecture beyond what is necessary to compile and test the workspace.

### Dependency Direction

The workspace must preserve the approved direction:

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
      ^                     |
      |                     v
      +---------------- crosslab-sim
```

Rules:

- no circular Cross-Lab dependencies;
- `crosslab-crypto` depends on no other Cross-Lab crate;
- UI, platform, transport, persistence, privileged-service, recovery-runtime, and plugin dependencies are absent;
- concrete Quinn/Iroh/libp2p types do not enter domain crates;
- no global models/services/controllers folder pattern is introduced.

### Tooling and CI

Establish the minimum automated quality baseline for the workspace:

- formatting check;
- Clippy across workspace/all targets/all features with warnings denied;
- workspace build/check;
- workspace tests.

A small GitHub Actions workflow may be added for these checks if the current stable toolchain and repository settings support it cleanly.

## Dependency Selection

Before adding third-party crates:

1. verify the current stable version rather than guessing;
2. inspect maintenance/upstream health;
3. check license compatibility with `MIT OR Apache-2.0`;
4. review security and transitive dependency cost;
5. add the dependency only when M1 actually requires it.

M1 should avoid production cryptography/protobuf/network dependencies unless a minimal compile boundary genuinely requires them. Functional dependency selection belongs with the milestone that first consumes the functionality.

Uploaded research repositories are references only. Do not copy their architecture or source into the foundation.

## Out of Scope

M1 does not implement:

- cryptographic signing, hashing, credential, or pairing behavior;
- identity/trust/policy domain models beyond minimal package boundaries;
- protobuf schemas or protocol framing;
- logical sessions;
- in-memory transport behavior;
- Quinn or any real networking;
- persistence/database selection;
- desktop/mobile applications;
- privileged helpers;
- recovery runtime;
- plugin runtime;
- capability implementations.

Those responsibilities begin in later Phase 1 milestones after the workspace baseline is verified.

## Acceptance Criteria

M1 is complete when:

- the repository has the approved minimal workspace layout;
- every package builds on the selected stable toolchain;
- package names and dependency direction match the architecture;
- no circular or speculative Cross-Lab dependencies exist;
- no unapproved platform/network/database/plugin structure is introduced;
- repository formatting, linting, build/check, and test commands pass;
- CI, when added, runs the same baseline checks;
- `docs/development/CURRENT.md` records the verified M1 checkpoint and identifies M2 as the next task.

## Verification

Run from the repository root:

```text
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Record any platform-specific exception explicitly rather than weakening the default verification baseline silently.

## Handoff

After M1 verification, proceed to **M2 — Crypto + Identity** using the accepted identity/crypto specifications and golden-vector obligations. Do not begin M2 behavior until the workspace foundation is clean and reproducible.
