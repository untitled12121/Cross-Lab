# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M6 — Authorized Data Streams: implementation complete on the feature branch; exact documentation-inclusive PR verification and merge are pending.**

M1–M5 are complete, verified, and integrated into canonical `main`. M6 Tasks 1–5 are implemented and the final code head has passed the full Rust baseline and bounded fuzz smoke.

## Branch State

- `main` — canonical through complete M5; final M5 integration-record head `c24f68342709efc72549f0bf8f3ea8df2ef6de48` passed CI `34662220773`.
- `m6-authorized-streams` — active M6 branch, based on `c24f68342709efc72549f0bf8f3ea8df2ef6de48` and represented by PR #16.
- Final M6 code head before this documentation checkpoint: `71ad5a620ff9c1c7be397a81a6adfddd2fc3cd20`.
- PR #16 had no submitted reviews or unresolved review threads at final code-head review.

## Architecture Baseline

- uploaded Cross-Lab Master Architecture & Development Plan — primary architectural source of truth;
- `docs/architecture/MASTER-ARCHITECTURE.md` — approved evolved repository architecture, Revision 2.1;
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 scenarios/milestone staging including S-007 and N-035..N-045;
- `docs/architecture/SESSION-TRANSPORT.md` — stream admission/transport contract;
- `docs/protocol/PROTOCOL-V1.md` — existing bounded `DataStreamOpenV1` wire contract;
- M3 `AuthorizedOperation` / `UsePolicy`;
- M5 `LogicalSession`, `SessionContext`, `TransportConnection`, `MemoryTransportPair`, and simulator composition.

## M6 Approved Architecture

- control-plane authorization remains separate from data-plane bytes;
- every accepted stream requires a locally held valid `AuthorizedOperation`;
- reuse existing M4 `DataStreamOpen` rather than inventing another header;
- `UsePolicy::MultiStream { max_streams: NonZeroU32 }` is bounded; there is no unlimited stream authorization;
- policy owns authority plus monotonic stream-use budget only;
- core owns session-bound stream admission, direction/index/replay state, operation registry, and admitted-stream lifecycle;
- transport carries opaque opening bytes and bounded chunks without interpreting authorization;
- simulator provides bounded stream opens/chunks, ownership-preserving backpressure, graceful finish, cancellation, and deterministic shutdown;
- no async runtime, real network transport, real files, UI/platform, persistence, or M7 reconnect/revocation implementation enters M6.

## M6 Planning Checkpoints

Written design: `docs/plans/phase-1/M6-authorized-data-streams-design.md`

- `1af29c57ff68b449ed991f8e372736da9ba9da42` — initial written design;
- `5a2bb9c3bbf5e9aa85d93dfc55b1599d4caa15ee` — refinement separating policy budget from core index/replay ownership.

Implementation plan: `docs/plans/phase-1/M6-authorized-data-streams.md`

- `9805347a4aad0da788554d40b371c4657cc7e37c` — initial implementation plan;
- `8faf3da38e57159dbbebd73611f958aba61e8376` — refinement making transport seam + memory adapter one atomic task and requiring simulator capacity checks before dequeue/admission.

Research inspected before implementation:

- uploaded Quinn — graceful send `finish`, abortive send `reset`, receive-side stop/cancellation lifecycle semantics;
- uploaded Iroh — unidirectional stream open/accept and graceful finish patterns;
- reuse is semantic only; no external transport type/code/dependency was introduced in M6.

## M6 Completed Implementation

### Task 1 — bounded stream-use authority

- Added bounded `MultiStream`, reservation accounting, stream-use rejection for `SingleAction`, budget exhaustion, and terminal/revision validation in `crosslab-policy`.
- Valid RED: `aa14692a3f46f65bb541c6f24cd475977a36d78b`, CI `34663985161` failed after the intended missing policy APIs were exercised.
- GREEN: `9a508d0934c7d9bc76861d61ef26482c46f61ced`, CI `34664047097` and Fuzz Smoke `34664047042` passed.

### Task 2 — session-bound core stream admission

- Added bounded operation registry/active-stream state, session/capability/version/direction/index binding, duplicate protection, operation reservation, finish/cancel lifecycle, and bounded MultiStream index tracking.
- Valid RED: `ecd08f358114eca6e0ab6f27ef703050e6870ed1`, CI `34664295772` passed lockfile/rustfmt and failed at workspace check for the intentionally missing admission implementation.
- GREEN/refinement head: `44ac89eb3b99ae3adba4f34c8526658eae763405`, CI `34664768007` and Fuzz Smoke `34664768009` passed.

### Task 3 — transport-neutral streams + bounded memory transport

- Extended the transport seam with unidirectional send/receive traits and explicit open/accept/send/receive errors while preserving unsent byte ownership and redacting payloads from `Debug`.
- Extended `MemoryTransportPair` with explicit bounded stream/open/chunk capacities, ordered delivery, graceful finish, cancellation, close propagation, and deterministic reclamation using only `Arc<Mutex<_>>` plus bounded `VecDeque` state.
- Valid RED: `d0615b88fc28951fb988435319d2a82b4ceab17d`, CI `34664820379` passed lockfile/rustfmt and failed at workspace check for the missing neutral stream APIs.
- Final Task 3 GREEN head: `7c500cf23560a697c6d633443f613fac8afe0994`, CI `34670304564` and Fuzz Smoke `34670304596` passed.

### Task 4 — simulator stream runtime + S-007

- Added `apps/sim/src/stream.rs` as composition glue only: protocol encode/decode stays in protocol, authorization/admission stays in core/policy, and queues stay in transport.
- S-007 proves `Allow -> AuthorizationGrant -> AuthorizedOperation(SingleStream) -> encoded open -> remote admission -> three ordered synthetic chunks -> graceful finish -> reuse rejection` with no real file I/O.
- Runtime saturation is checked before transport dequeue so a pending stream and its operation budget remain untouched until capacity is available.
- Valid RED: `3c0d8fb16cf0e543bd991b0577b8bcbe5f1d2287`, CI `34670774976` passed lockfile/rustfmt and failed at workspace check only because `crosslab_sim::stream` did not yet exist.
- GREEN: `ecc07ed513b89229a8e61441010b8af1a7256912`, CI `34670944091` and Fuzz Smoke `34670944110` passed.

### Task 5 — security/lifecycle closeout

The completed test matrix explicitly covers:

- N-035 — no registered `OperationId` rejects before payload exposure;
- N-036 — expired/cancelled/revoked/consumed operations reject;
- N-037 — wrong session, authenticated peer binding, capability, version, operation, and direction reject;
- N-038 — second `SingleStream` use rejects;
- N-039 — trust/policy revision changes reject;
- N-043 — pending-open, chunk, and simulator-runtime saturation produce bounded backpressure/resource errors;
- N-045 — malformed setup and admission failure cancel the receive side without dangling admitted/runtime state;
- shutdown — active memory streams are cancelled, admission authority is cancelled, and the memory transport closes;
- `MultiStream(2)` — indices 1 then 0 succeed; duplicate index and index 2 fail.

Final code head `71ad5a620ff9c1c7be397a81a6adfddd2fc3cd20` passed:

- CI `34671218918`: locked metadata, rustfmt, workspace check, Clippy with `-D warnings`, and all workspace tests;
- Fuzz Smoke `34671218922`: success on the existing bounded parser targets, including the M6-owned `data_stream_open` parser surface.

No new parser was introduced, so no fuzz target/workflow modification was justified.

## Final Scope Review

PR #16 changes only these M6 areas:

- policy stream-use budgets and tests;
- core stream admission and transport stream seam plus tests;
- bounded in-memory simulator stream transport/runtime plus tests;
- M6 design/implementation documentation and this resume guide.

The final diff contains no Quinn/Iroh/libp2p integration, sockets/TLS, async runtime, real files/resume/sync, persistence, UI/platform adapters, plugins, privileged services, or M7 lifecycle implementation.

## Exact Next Task

1. Verify this documentation-inclusive PR head with locked metadata, rustfmt, workspace check, Clippy `-D warnings`, all workspace tests, and Fuzz Smoke.
2. Mark PR #16 ready and merge only that exact verified head to `main` using preserved-history merge with expected-head protection.
3. Verify canonical post-merge `main` CI and confirm the merge tree matches the fuzz-verified PR tree.
4. Write the final M6 integration record on `main`, verify that documentation-only head, then begin **M7 — disconnect/reconnect/revocation/failure simulator** from the verified canonical `main` state.

Do not start M7 implementation before the M6 merge and final canonical checkpoint are verified.

## Resume Procedure

1. inspect `main`, `m6-authorized-streams`, recent commits, PR/workflow state, and repository cleanliness;
2. read the Master Architecture, this file, the active milestone plan, and relevant ADRs/architecture contracts;
3. reconcile docs with actual code before editing;
4. work in small verified milestones and merge only exact verified heads to `main`;
5. never rely on chat history or stash as the only copy of incomplete work.
