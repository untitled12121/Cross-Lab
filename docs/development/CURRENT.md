# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M6 — Authorized Data Streams: design drafted and self-reviewed; awaiting written design approval.**

M1–M5 are complete, verified, and integrated into canonical `main`. No M6 production code has been written.

## Branch State

- `main` — canonical branch through complete M5; final integration-record head `c24f68342709efc72549f0bf8f3ea8df2ef6de48` passed CI `34662220773`.
- `m6-authorized-streams` — active M6 design/planning branch created exactly from `c24f68342709efc72549f0bf8f3ea8df2ef6de48`.
- Historical M5/planning/protocol/trust-policy branches were verified as fully contained in `main` and removed by the repository owner before M6 planning began.

## Architecture Baseline

- uploaded Cross-Lab Master Architecture & Development Plan — primary architectural source of truth;
- `docs/architecture/MASTER-ARCHITECTURE.md` — approved evolved repository architecture, Revision 2.1;
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator specification and milestone staging;
- `docs/architecture/SESSION-TRANSPORT.md` — logical-session/data-stream admission and transport contract;
- `docs/protocol/PROTOCOL-V1.md` — existing `DataStreamOpenV1` and framing contract;
- M3 `AuthorizedOperation` / `UsePolicy` implementation;
- M5 `LogicalSession`, `SessionContext`, `TransportConnection`, `MemoryTransportPair`, and `SimNode` implementation.

## Completed M5 Baseline

M5 proves pairing, trust commit, bounded in-memory control transport, channel-bound authenticated logical sessions, capability exchange, and sequenced policy-authorized control request/response/event simulation.

Key integration evidence:

- Tasks 1–4 PR #10: final head `588366d69dcd89d3a9a4f709fbca833eb263be57`; CI `34643647466`; fuzz `34643647458`; merged at `b54395109b8fe7ed49862f7f8c508659a2fd7a36`.
- Task 5 PR #11: final head `8ad7c79eca6e8aa70c887d27d3a66aa713df9d9f`; CI `34653899890`; fuzz `34653899783`; merged at `8cb6f013055a4f05ff599cc0d31adac45d2756b4`.
- Task 6 PR #12: final head `9f2fcaf157245b6dd1f1ab1083e9507961cc7e8a`; CI `34654964311`; merged at `32bf75951c0e0e768efac80c4320059e04550483`.
- Task 7 PR #13: documentation-inclusive head `94f418469a24dcd0d9598311a1d20bfda1bbef3a`; CI `34658806668`; fuzz `34658806735`; merged at `e17075245b566522bb4c822cc22b3ee2245fe7a7`.
- Task 8 PR #14: documentation-inclusive head `212212e4a353abf7d4b505dd633c9f95a04e3027`; CI `34660116960`; merged at `b14f4c4a7b42b22c1c8de23be4c8c518e24dbc34`.
- Task 9 PR #15: exact code head `8a4ae668d6cbfd329d72914d8b47279263cf14d7` passed CI `34661829192` and fuzz `34661829233`; documentation-inclusive head `654683b62cde22cdd528920fcebeec64d6ca3119` passed CI `34661982660` and fuzz `34661982649`; merged at `e9ddec90b9ad232c917c302188400abb23a39c54`; post-merge CI `34662096131` passed.
- Final M5 integration-record head `c24f68342709efc72549f0bf8f3ea8df2ef6de48` passed CI `34662220773`.

The M5 merged tree is the exact fuzz-verified PR tree; Fuzz Smoke is PR/manual-triggered rather than push-triggered.

## M6 Approved Direction

The repository owner approved the M6 direction in chat before the written spec was created:

- control-plane authorization remains separate from data-plane bytes;
- every stream requires a locally held valid `AuthorizedOperation`;
- reuse existing M4 `DataStreamOpen` rather than invent a parallel header;
- add bounded multi-stream authority as `MultiStream { max_streams: NonZeroU32 }`, never unlimited reuse;
- policy owns only operation authority and monotonic stream-use budget;
- core owns session-bound stream admission, direction/index semantics, operation registry, and active stream lifecycle;
- transport carries opaque opening bytes and bounded chunks without interpreting protocol authority;
- simulator provides bounded pending-open/chunk queues, backpressure, graceful finish, cancellation, and deterministic shutdown;
- no general async runtime, real networking, filesystem transfer, UI/platform, persistence, or M7 lifecycle work enters M6.

## M6 Design Checkpoint

Written design:

`docs/plans/phase-1/M6-authorized-data-streams-design.md`

Design commits on `m6-authorized-streams`:

- `1af29c57ff68b449ed991f8e372736da9ba9da42` — initial written design;
- `5a2bb9c3bbf5e9aa85d93dfc55b1599d4caa15ee` — self-review refinement separating policy stream-use budget from core stream-index/replay ownership and avoiding permanent session-wide `StreamId` history.

No production source, Cargo dependency, protocol schema, or workflow file changed in these commits.

## Exact Next Task

Do **not** implement M6 yet.

1. repository owner reviews `docs/plans/phase-1/M6-authorized-data-streams-design.md` and explicitly approves it or requests changes;
2. after written design approval, invoke the planning workflow and create the detailed M6 implementation plan in `docs/plans/phase-1/`;
3. self-review the implementation plan against the Master Architecture, `CORE-SIMULATOR.md`, `SESSION-TRANSPORT.md`, current code, and relevant uploaded research repositories;
4. only after plan approval begin TDD implementation on `m6-authorized-streams`;
5. use small verifiable slices, update this file after meaningful progress, and merge only exact verified work into `main`.

## M6 Acceptance Guardrails

M6 must prove operation-bound stream admission, single-stream and bounded multi-stream budgets, no payload visibility before admission, bounded open/chunk queues, ownership-preserving backpressure, cancellation/shutdown, S-007 and applicable N-035..N-039/N-043/N-045 behavior, and the full Rust/fuzz baseline.

M6 explicitly excludes Quinn/Iroh/libp2p, real sockets/TLS, real files/resume/sync, UI/platform adapters, persistence, plugins, privileged services, and M7 reconnect/revocation orchestration.

## Resume Procedure

Before continuing in a new chat:

1. inspect `main`, `m6-authorized-streams`, recent commits, PR/workflow state, and repository cleanliness;
2. read the Master Architecture, this file, the M6 design, `CORE-SIMULATOR.md`, and `SESSION-TRANSPORT.md`;
3. reconcile docs with actual code before editing;
4. respect the current written-design review gate before producing the implementation plan or code;
5. never rely on chat history or stash as the only copy of incomplete work.
