# Current Development State

This file is the resume guide for Cross-Lab. Git history, repository contents, and verification results remain the factual implementation state if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator: Ready to Start**

Phase 0 — Architecture and Security Specification is complete and integrated into `main`.

## Current Milestone

**M1 — Repository Foundation**

No Phase 1 production code has been written yet.

## Active Branch

`main`

The completed `planning` branch is retained as the Phase 0 planning ref. Create `foundation` from the integrated `main` baseline before beginning M1 implementation.

## Last Verified Phase 0 Checkpoint

`11b19b9a7cdb135207f1fc78896d74a9ef7dc108` — completed Phase 0 architecture planning and closeout baseline, fast-forward integrated into `main`.

## Completed

- P0.1-P0.9 foundational architecture/security milestones.
- Threat model and security-boundary specification.
- Owner/device identity and key hierarchy.
- Pairing, trust establishment, and revocation model.
- Default-deny policy and operation-bound authorization model.
- Protocol v1 compatibility, bounded framing, lifecycle, replay/retry/cancellation, and canonical signing transcript.
- Secure logical-session and transport-neutral contract.
- Recovery/update security model, audit/privacy rules, and reserved plugin authority boundary.
- Core Simulator workspace/scenario specification.
- ADR-0001 through ADR-0006 accepted and reconciled with Master Architecture Revision 2.1.
- Repository license fixed as `MIT OR Apache-2.0`.
- Phase 0 milestone records professionalized and duplicate P0.2 planning documents consolidated.
- Phase 0 closeout review completed.
- Purpose-based branch workflow established.

## Deferred Decisions

These are intentional later-milestone choices and are not blockers for M1:

- remote NAT/relay implementation beyond the Quinn baseline;
- persistent metadata database;
- dedicated transport-abstraction crate;
- plugin runtime;
- CRDT use;
- TCP/TLS fallback and WireGuard integration;
- platform-specific privileged IPC and driver/kernel components;
- updater implementation details;
- platform-native credential profiles beyond v1;
- low-entropy numeric pairing profile.

## Verification

Phase 0 was documentation/specification-only; no Rust workspace existed, so Rust format/lint/test commands were not applicable.

Verified at closeout:

- `planning` preserved all commits formerly unique to `foundation/phase-0-to-1` and added one clean closeout commit before integration;
- P0.1-P0.9 milestone records no longer contain internal agent/sub-skill execution instructions;
- P0.2 duplicate planning files were consolidated;
- Master Architecture Revision 2.1 reflects the accepted Phase 0 ADR decisions;
- license files are present;
- `README.md`, `CONTRIBUTING.md`, `WORKFLOW.md`, `ROADMAP.md`, and the Phase 0 closeout record agree on the implementation handoff;
- no production source code or third-party research source adaptation was introduced by Phase 0.

## Exact Next Task

1. Create `foundation` from the current `main` commit.
2. Re-read `docs/architecture/CORE-SIMULATOR.md`, Master Architecture Sections 36-45 and 49, and ADR-0006.
3. Inspect the uploaded research repositories relevant to the initial Rust foundation only where they can inform dependency/API choices; do not copy architecture blindly.
4. Verify current stable Rust/dependency versions required for M1 instead of guessing them.
5. Create the minimal Rust workspace with `crosslab-crypto`, `crosslab-identity`, `crosslab-policy`, `crosslab-protocol`, `crosslab-core`, and `crosslab-sim` shells, plus formatting/lint/test configuration.
6. Run formatting, Clippy, build, and workspace tests before the M1 checkpoint.

## Resume Procedure

In a new session:

1. Inspect `main`, branch refs, and recent commits.
2. Read `docs/architecture/MASTER-ARCHITECTURE.md`.
3. Read this file.
4. Read `docs/architecture/CORE-SIMULATOR.md` and relevant ADRs.
5. Create or inspect the `foundation` branch.
6. Continue from the exact next task above, reconciling the file with actual repository state first.
