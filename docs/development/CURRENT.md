# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

Phase 0 — Architecture and Security Specification is complete and integrated into `main`.

## Current Milestone

**M1 — Repository Foundation**

Status: ready for implementation. No Phase 1 production code has been written yet.

## Canonical Branch

`main`

`main` is the integrated source branch. The `planning` branch is used only for planning/documentation changes and must be merged back into `main` before implementation depends on those changes.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- ADR-0001 through ADR-0006 — Accepted.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator specification.
- `docs/plans/phase-1/M1-repository-foundation.md` — active milestone plan.

## Completed Foundation

Phase 0 established the required implementation contracts for:

- threat model and security boundaries;
- owner/device identity and key hierarchy;
- pairing, trust establishment, and revocation;
- capability, policy, obligation, and operation-scoped authorization;
- Protocol v1 compatibility, framing, lifecycle, replay/retry/cancellation, and canonical signing transcripts;
- secure logical sessions, channel binding, reconnect, and transport-neutral boundaries;
- recovery authority, update trust, audit/privacy, and reserved plugin security boundaries;
- deterministic Core Simulator responsibilities and required security scenarios;
- repository license (`MIT OR Apache-2.0`) and focused `crosslab-crypto` boundary.

Phase 0 closeout: `docs/plans/phase-0/PHASE-0-CLOSEOUT.md`.

## Deferred Decisions

The following are intentionally later-milestone choices and do not block M1:

- remote NAT/relay design beyond the Quinn baseline;
- persistent metadata/database selection;
- dedicated transport-abstraction crate until multiple transports justify extraction;
- plugin runtime and stable plugin ABI;
- CRDT adoption;
- TCP/TLS fallback and WireGuard integration;
- platform-specific privileged IPC, drivers, and kernel components;
- updater implementation dependency/packaging details;
- platform-native credential profiles beyond v1;
- low-entropy numeric pairing profile.

## Verification State

Phase 0 was documentation/specification-only, so Rust format/lint/build/test commands were not applicable.

The Phase 0 baseline has been reviewed for:

- accepted ADR status and Master Architecture consistency;
- repository license materialization;
- completed P0.1-P0.9 milestone records and closeout coverage;
- explicit Phase 1 entry conditions;
- absence of production source code or unreviewed research-source adaptation.

M1 introduces the Rust verification baseline.

## Exact Next Task

Implement `docs/plans/phase-1/M1-repository-foundation.md`:

1. verify the current stable Rust toolchain and any dependency versions actually required by M1;
2. create the minimal Rust workspace defined by Master Architecture Revision 2.1;
3. add package shells for `crosslab-crypto`, `crosslab-identity`, `crosslab-policy`, `crosslab-protocol`, `crosslab-core`, and `crosslab-sim` with the approved dependency direction;
4. establish formatting, Clippy, build/check, and test automation;
5. run the complete M1 verification baseline before marking the milestone complete;
6. update this file with the verified M1 checkpoint and exact M2 task.

Do not introduce Phase 2/platform features, real networking, persistence, plugins, or privileged services during M1.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, `planning`, recent commits, and repository state;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read this file;
4. read `docs/plans/phase-1/M1-repository-foundation.md` and `docs/architecture/CORE-SIMULATOR.md`;
5. read ADR-0006 and any other ADR relevant to the exact implementation step;
6. reconcile documentation with actual code and verification state;
7. continue from the exact next task above.
