# Current Development State

This file is the resume guide for Cross-Lab. Git history, repository contents, and verification results remain the factual implementation state if this document becomes stale.

## Current Phase

**Phase 0 — Architecture and Security Specification: Complete**

## Current Milestone

**Phase 0 closeout and integration**

The Phase 0 architecture/security baseline is complete and ready to integrate into `main`.

## Active Branch

`planning`

`planning` is the sole active Phase 0 work branch. Historical branch refs are not part of the active workflow.

## Last Verified Checkpoint

The Phase 0 closeout commit containing this file is the current verified planning checkpoint. Its preserved Phase 0 specification ancestry includes `6b5b26e0324ad586db88afa68f744bf7f50984fd`.

## Completed

- P0.1-P0.9 foundational planning/specification milestones.
- Threat model and security-boundary specification.
- Owner/device identity and key hierarchy.
- Pairing, trust establishment, and revocation model.
- Default-deny policy and operation-bound authorization model.
- Protocol v1 compatibility, framing, lifecycle, replay/retry/cancellation, and canonical signing transcript.
- Secure logical-session and transport-neutral contract.
- Recovery/update security model, audit/privacy rules, and reserved plugin authority boundary.
- Core Simulator workspace/scenario specification.
- Phase 0 ADR decisions and repository licensing.
- Professionalized milestone records and formal Phase 0 closeout review.
- Purpose-based branch workflow for future development.

## Accepted Phase 0 ADRs

- ADR-0001 — `MIT OR Apache-2.0` repository license
- ADR-0002 — identity cryptographic profile v1
- ADR-0003 — pairing bootstrap profile v1
- ADR-0004 — Protocol Buffers + canonical signing transcript v1
- ADR-0005 — TUF-style update trust model v1
- ADR-0006 — focused `crosslab-crypto` Phase 1 foundation crate

## Known Deferred Decisions

These are intentionally deferred and are not blockers for Phase 1:

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

Phase 0 is documentation/specification-only. No Rust workspace exists yet, so `cargo fmt`, Clippy, and Rust tests are not applicable to this checkpoint.

Closeout verification requires:

- all former `foundation/phase-0-to-1` work preserved on `planning`;
- P0.1-P0.9 plans contain no internal agent/sub-skill instructions;
- P0.2 duplicate planning files consolidated;
- accepted ADRs and architecture baseline aligned;
- license files present;
- `WORKFLOW.md`, `ROADMAP.md`, this file, and the closeout review agree on the next milestone;
- no production code, secrets, or third-party source adaptation introduced during Phase 0.

## Exact Next Task

1. Integrate the verified `planning` closeout into `main`.
2. Create a clean `foundation` branch from the integrated `main` commit.
3. Begin **Phase 1 / M1 — Repository Foundation** by creating the minimal Rust workspace and simulator shell defined by `docs/architecture/CORE-SIMULATOR.md`.
4. Verify current stable dependency versions before adding dependencies.
5. Establish formatting, linting, build, and test checks in the first implementation milestone.

## Resume Procedure

In a new session:

1. Read `docs/architecture/MASTER-ARCHITECTURE.md`.
2. Read this file.
3. Read `docs/plans/phase-0/PHASE-0-CLOSEOUT.md` for the frozen Phase 0 baseline.
4. Read the ADRs relevant to the first implementation slice.
5. Inspect `main` and the active implementation branch before modifying code.
6. Continue from the exact next task above unless repository state shows it has already been completed.
