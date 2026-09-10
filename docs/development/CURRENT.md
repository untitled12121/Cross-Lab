# Current Development State

This file is the resume guide for active Cross-Lab development. Git history, repository contents, and tests are the factual implementation state when they conflict with this document.

## Current phase

**Phase 0 — Architecture and Security Specification**

## Current milestone

**P0.1 — Project Foundation**

Status: implementation complete; final repository review and pull-request checkpoint in progress.

## Active branch

`phase-0/project-foundation`

## Last verified content commit

`0119de756ae93f333fad0872d4b3fb4bc0a0270f` — authoritative Architecture Baseline Revision 2.0 mirrored into the repository.

The architecture file's Git blob SHA is `069f0014e06af8ccabfc9d06d4bf46241b3f8e82`, matching the approved source file exactly.

## Completed

- Added the P0.1 implementation plan.
- Created the project README and focused repository ignore rules.
- Added contributor and security baseline documentation.
- Added the Git-first development continuity and interruption-recovery workflow.
- Added a concise roadmap that points back to the Master Architecture.
- Established ADR naming, lifecycle, and required-content rules.
- Mirrored the approved Cross-Lab Master Architecture & Development Plan Revision 2.0 into Git without content drift.
- Kept production source code, Rust workspace scaffolding, platform code, transport dependencies, and research-source adaptations out of P0.1.

## In progress

- Final P0.1 repository review.
- Pull request from `phase-0/project-foundation` to `main`.

## Exact next task

After P0.1 is reviewed and merged, start **P0.2 — Threat Model and Trust Boundaries**.

P0.2 must derive the initial threat model from the architectural invariants and boundaries in Sections 4, 6, 8, 12, 20, 32, 33, and 42 of `docs/architecture/MASTER-ARCHITECTURE.md` before cryptographic or protocol implementation begins.

## Known issues / open decisions

- The Cross-Lab repository license is not yet selected. No third-party research source code may be adapted while this remains unresolved.
- Remote NAT/relay architecture remains intentionally unresolved until the M9 networking evaluation checkpoint.
- Final wire serialization and canonical signing interaction remain Phase 0 decisions.

## Verification

P0.1 is documentation-only; no Rust build/test suite exists yet.

Completed checks:

- Architecture source mirror: **PASS** — Git blob SHA matches the approved Revision 2.0 source exactly.
- Production implementation claims review: **PASS** — documentation states that production implementation has not started.
- Dependency/source introduction review: **PASS** — no production dependencies or research-source adaptations introduced.
- Continuity review: **PASS** — branch, milestone, completed work, open work, verification state, and exact next task are documented.

Not applicable in P0.1:

- `cargo fmt --check`
- `cargo clippy`
- `cargo test`

These become mandatory when the Rust workspace is introduced.

## Active plan

`docs/plans/phase-0/P0.1-project-foundation.md`

## Relevant ADRs

None accepted yet. ADR governance is defined in `docs/adr/README.md`.

## Resume procedure

Before continuing in any new session:

1. Read `docs/architecture/MASTER-ARCHITECTURE.md`.
2. Read this file.
3. Read the active plan.
4. Read relevant ADRs.
5. Inspect the active branch and recent commits.
6. Reconcile this document with actual repository state.
7. Continue only from the exact next task or document any divergence first.
