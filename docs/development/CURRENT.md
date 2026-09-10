# Current Development State

This file is the resume guide for active Cross-Lab development. Git history, repository contents, and tests are the factual implementation state when they conflict with this document.

## Current phase

**Phase 0 — Architecture and Security Specification**

## Current milestone

**P0.1 — Project Foundation**

Status: complete on the feature branch and ready for integration through Pull Request #1.

## Active branch

`phase-0/project-foundation`

## Pull request

`#1 — P0.1: establish project foundation`

Base: `main`  
Head: `phase-0/project-foundation`

## Last verified content checkpoint

`a6227e8abf60ec4d640bd3a2864178e439dec44e` — P0.1 implementation plan marked complete after repository review and PR creation.

The authoritative architecture file's Git blob SHA is `069f0014e06af8ccabfc9d06d4bf46241b3f8e82`, exactly matching the approved Cross-Lab Master Architecture & Development Plan Revision 2.0 source.

## Completed

- Added the P0.1 implementation plan and completed its checklist.
- Created the project README and focused repository ignore rules.
- Added contributor and security baseline documentation.
- Added the Git-first development continuity and interruption-recovery workflow.
- Added a concise roadmap that points back to the Master Architecture.
- Established ADR naming, lifecycle, and required-content rules.
- Mirrored the approved Cross-Lab Master Architecture & Development Plan Revision 2.0 into Git without content drift.
- Reviewed the branch against `main` and confirmed the P0.1 scope contains documentation/governance only.
- Opened Pull Request #1 to integrate P0.1 into `main`.
- Kept production source code, Rust workspace scaffolding, platform code, transport dependencies, and research-source adaptations out of P0.1.

## In progress

No P0.1 implementation work remains. Integration of Pull Request #1 is pending the repository integration decision.

## Exact next task

After P0.1 is integrated, start **P0.2 — Threat Model and Trust Boundaries** on a new feature branch.

P0.2 must derive the initial threat model from Sections 4, 6, 8, 12, 20, 32, 33, and 42 of `docs/architecture/MASTER-ARCHITECTURE.md` before cryptographic or protocol implementation begins.

The P0.2 deliverable should define protected assets, trust assumptions, attacker classes, trust boundaries, privileged boundaries, recovery/update boundaries, major abuse cases, required mitigations, and security invariants without prematurely selecting unresolved cryptographic or wire-format details.

## Known issues / open decisions

- The Cross-Lab repository license is not yet selected. No third-party research source code may be adapted while this remains unresolved.
- Remote NAT/relay architecture remains intentionally unresolved until the M9 networking evaluation checkpoint.
- Final wire serialization and canonical signing interaction remain Phase 0 decisions.

## Verification

P0.1 is documentation-only; no Rust build/test suite exists yet.

Completed checks:

- Architecture source mirror: **PASS** — Git blob SHA matches the approved Revision 2.0 source exactly.
- Repository structure review: **PASS** — required P0.1 files exist on the feature branch.
- Production implementation claims review: **PASS** — documentation states that production implementation has not started.
- Dependency/source introduction review: **PASS** — no production dependencies or research-source adaptations introduced.
- Continuity review: **PASS** — branch, PR, milestone, completed work, open decisions, verification state, and exact next task are documented.
- Branch comparison before final handoff: **PASS** — feature branch was ahead of `main` with no behind commits.

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
5. Inspect the active branch, Pull Request #1, and recent commits.
6. Reconcile this document with actual repository state.
7. Integrate P0.1 or document why integration is deferred.
8. Start P0.2 only after the P0.1 integration state is clear.
