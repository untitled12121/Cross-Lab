# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M6 — Authorized Data Streams: design approved; implementation plan complete and self-reviewed; implementation not started.**

M1–M5 are complete, verified, and integrated into canonical `main`. No M6 production source, dependency, protocol schema, or workflow behavior has been changed yet.

## Branch State

- `main` — canonical through complete M5; final integration-record head `c24f68342709efc72549f0bf8f3ea8df2ef6de48` passed CI `34662220773`.
- `m6-authorized-streams` — active M6 design/planning branch created exactly from `c24f68342709efc72549f0bf8f3ea8df2ef6de48`.
- Historical M5/planning/protocol/trust-policy branches were verified as fully contained in `main` and removed by the repository owner before M6 planning.

## Architecture Baseline

- uploaded Cross-Lab Master Architecture & Development Plan — primary architectural source of truth;
- `docs/architecture/MASTER-ARCHITECTURE.md` — approved evolved repository architecture, Revision 2.1;
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 scenarios/milestone staging including S-007 and N-035..N-045;
- `docs/architecture/SESSION-TRANSPORT.md` — stream admission/transport contract;
- `docs/protocol/PROTOCOL-V1.md` — existing bounded `DataStreamOpenV1` wire contract;
- M3 `AuthorizedOperation` / `UsePolicy`;
- M5 `LogicalSession`, `SessionContext`, `TransportConnection`, `MemoryTransportPair`, and simulator composition.

## Completed M5 Baseline

M5 final Task 9 PR head `654683b62cde22cdd528920fcebeec64d6ca3119` passed CI `34661982660` and Fuzz Smoke `34661982649`; PR #15 merged at `e9ddec90b9ad232c917c302188400abb23a39c54`; post-merge CI `34662096131` passed; final integration-record head `c24f68342709efc72549f0bf8f3ea8df2ef6de48` passed CI `34662220773`.

The merged M5 code tree is the exact tree fuzz-verified on the PR head.

## M6 Approved Architecture

- control-plane authorization remains separate from data-plane bytes;
- every accepted stream requires a locally held valid `AuthorizedOperation`;
- reuse existing M4 `DataStreamOpen` rather than inventing another header;
- add `MultiStream { max_streams: NonZeroU32 }`, never unlimited reuse;
- policy owns authority plus monotonic stream-use budget only;
- core owns session-bound stream admission, direction/index/replay state, operation registry, and admitted-stream lifecycle;
- transport carries opaque opening bytes and bounded chunks without interpreting authorization;
- simulator provides bounded stream opens/chunks, ownership-preserving backpressure, graceful finish, cancellation, and deterministic shutdown;
- no async runtime, real network transport, real files, UI/platform, persistence, or M7 reconnect/revocation work enters M6.

## M6 Planning Checkpoints

Written design:

`docs/plans/phase-1/M6-authorized-data-streams-design.md`

- `1af29c57ff68b449ed991f8e372736da9ba9da42` — initial written design;
- `5a2bb9c3bbf5e9aa85d93dfc55b1599d4caa15ee` — self-review refinement separating policy budget from core index/replay ownership.

Implementation plan:

`docs/plans/phase-1/M6-authorized-data-streams.md`

- `9805347a4aad0da788554d40b371c4657cc7e37c` — initial implementation plan;
- `8faf3da38e57159dbbebd73611f958aba61e8376` — plan self-review refinement: transport seam + memory adapter made one atomic task; simulator capacity checked before dequeue/admission so saturation cannot spend operation authority.

Research inspected before planning:

- uploaded Quinn: graceful send `finish`, abortive send `reset`, receive-side `stop` lifecycle semantics;
- uploaded Iroh: uni-stream `open_uni` / `accept_uni` and graceful finish patterns;
- reuse is semantic only; no external transport type/code/dependency is introduced in M6.

## M6 Implementation Plan

Five TDD tasks:

1. bounded stream-use authority in `crosslab-policy`;
2. session-bound `StreamAdmission` in `crosslab-core`;
3. transport-neutral stream seam plus bounded `MemoryTransportPair` streams as one atomic compile-safe slice;
4. feature-focused simulator stream runtime plus S-007;
5. N-035..N-039/N-043/N-045, shutdown, fuzz/baseline, scope review, merge, and M7 handoff.

## Exact Next Task

Begin **M6 Task 1 — bounded stream-use authority** only after selecting the implementation execution mode.

Task 1 starts test-first in `crates/policy/tests/operations.rs`, verifies RED for the missing `MultiStream`/reservation APIs, implements the smallest policy budget state in `crates/policy/src/operation/mod.rs`, runs focused plus full workspace gates, commits the green slice, and updates this file with exact evidence.

Do not start Task 2 until Task 1 is verified.

## M6 Acceptance Guardrails

M6 must prove operation-bound admission, single-stream and bounded multi-stream budgets, no payload visibility before admission, bounded open/live/chunk state, ownership-preserving backpressure, cancellation/shutdown, S-007, applicable N-035..N-039/N-043/N-045 behavior, and full Rust/fuzz verification.

M6 excludes Quinn/Iroh/libp2p integration, sockets/TLS, real files/resume/sync, UI/platform adapters, persistence, plugins, privileged services, and M7 reconnect/revocation orchestration.

## Resume Procedure

1. inspect `main`, `m6-authorized-streams`, recent commits, PR/workflow state, and repository cleanliness;
2. read the Master Architecture, this file, M6 design, M6 implementation plan, `CORE-SIMULATOR.md`, and `SESSION-TRANSPORT.md`;
3. reconcile docs with actual code before editing;
4. execute M6 task-by-task with TDD and exact-head verification; merge only verified work to `main`;
5. never rely on chat history or stash as the only copy of incomplete work.
