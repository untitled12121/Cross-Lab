# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M6 — Authorized Data Streams: complete, verified, and integrated into canonical `main`.**

M1–M6 are complete and integrated. The next milestone is **M7 — disconnect/reconnect/revocation/failure simulator**. Do not begin M7 implementation before reading the current architecture, existing code, and M7-relevant contracts and writing or confirming its implementation plan.

## Branch State

- `main` — canonical M6 merge commit `4d73528329fdae5d3eb1c25279778973168c87bd` passed post-merge CI `34671715166`.
- M6 exact verified PR head `b429c3e853477b874d1f7b8ec366f7acfa1248e7` passed CI `34671648182` and Fuzz Smoke `34671648146` before merge.
- PR #16 merged with preserved history and expected-head protection into `main` at `4d73528329fdae5d3eb1c25279778973168c87bd`.
- The verified PR head and merge commit have the identical Git tree `a5992db1c6ffd6f1b7335f465fdd76dd48943880`, so canonical M6 code is byte-identical to the fuzz-verified PR tree.
- `m6-authorized-streams` still exists at `b429c3e853477b874d1f7b8ec366f7acfa1248e7`; no branch-cleanup claim is implied by this record.

## Architecture Baseline

- uploaded Cross-Lab Master Architecture & Development Plan — primary architectural source of truth;
- `docs/architecture/MASTER-ARCHITECTURE.md` — approved evolved repository architecture, Revision 2.1;
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 scenarios/milestone staging;
- `docs/architecture/SESSION-TRANSPORT.md` — session, stream admission, transport, and failure-boundary contract;
- `docs/protocol/PROTOCOL-V1.md` — bounded protocol wire contract including `DataStreamOpenV1`;
- M3 `AuthorizedOperation` / `UsePolicy`;
- M5 authenticated `LogicalSession`, `SessionContext`, control runtime, and bounded memory transport;
- M6 bounded operation stream budgets, session-bound stream admission, transport-neutral data streams, and simulator stream runtime.

## M6 Approved Architecture

- control-plane authorization remains separate from data-plane bytes;
- every accepted stream requires a locally held valid `AuthorizedOperation`;
- existing M4 `DataStreamOpen` is reused rather than introducing another opening header;
- `UsePolicy::MultiStream { max_streams: NonZeroU32 }` is bounded; there is no unlimited stream authorization;
- policy owns authority plus monotonic stream-use budget only;
- core owns session-bound stream admission, direction/index/replay state, operation registry, and admitted-stream lifecycle;
- transport carries opaque opening bytes and bounded chunks without interpreting authorization;
- simulator provides bounded stream opens/chunks, ownership-preserving backpressure, graceful finish, cancellation, and deterministic shutdown;
- no Quinn/Iroh/libp2p integration, sockets/TLS, async runtime, real files, persistence, UI/platform, privileged services, or M7 reconnect/revocation implementation entered M6.

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
- reuse was semantic only; no external transport type/code/dependency was introduced in M6.

## M6 Completed Implementation

### Task 1 — bounded stream-use authority

- Added bounded `MultiStream`, reservation accounting, stream-use rejection for `SingleAction`, budget exhaustion, and terminal/revision validation in `crosslab-policy`.
- Valid RED: `aa14692a3f46f65bb541c6f24cd475977a36d78b`, CI `34663985161`.
- GREEN: `9a508d0934c7d9bc76861d61ef26482c46f61ced`, CI `34664047097`, Fuzz Smoke `34664047042`.

### Task 2 — session-bound core stream admission

- Added bounded operation registry/active-stream state, session/capability/version/direction/index binding, duplicate protection, operation reservation, finish/cancel lifecycle, and bounded MultiStream index tracking.
- Valid RED: `ecd08f358114eca6e0ab6f27ef703050e6870ed1`, CI `34664295772` passed lockfile/rustfmt and failed at workspace check for the intentionally missing admission implementation.
- GREEN/refinement: `44ac89eb3b99ae3adba4f34c8526658eae763405`, CI `34664768007`, Fuzz Smoke `34664768009`.

### Task 3 — transport-neutral streams + bounded memory transport

- Extended the transport seam with unidirectional send/receive traits and explicit open/accept/send/receive errors while preserving unsent byte ownership and redacting payloads from `Debug`.
- Extended `MemoryTransportPair` with explicit bounded stream/open/chunk capacities, ordered delivery, graceful finish, cancellation, close propagation, and deterministic reclamation using only `Arc<Mutex<_>>` plus bounded `VecDeque` state.
- Valid RED: `d0615b88fc28951fb988435319d2a82b4ceab17d`, CI `34664820379` passed lockfile/rustfmt and failed at workspace check for the missing neutral stream APIs.
- GREEN: `7c500cf23560a697c6d633443f613fac8afe0994`, CI `34670304564`, Fuzz Smoke `34670304596`.

### Task 4 — simulator stream runtime + S-007

- Added `apps/sim/src/stream.rs` as composition glue only: protocol encode/decode stays in protocol, authorization/admission stays in core/policy, and queues stay in transport.
- S-007 proves `Allow -> AuthorizationGrant -> AuthorizedOperation(SingleStream) -> encoded open -> remote admission -> three ordered synthetic chunks -> graceful finish -> reuse rejection` with no real file I/O.
- Runtime saturation is checked before transport dequeue so a pending stream and its operation budget remain untouched until capacity is available.
- Valid RED: `3c0d8fb16cf0e543bd991b0577b8bcbe5f1d2287`, CI `34670774976` passed lockfile/rustfmt and failed at workspace check only because `crosslab_sim::stream` did not yet exist.
- GREEN: `ecc07ed513b89229a8e61441010b8af1a7256912`, CI `34670944091`, Fuzz Smoke `34670944110`.

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

Final code/test head `71ad5a620ff9c1c7be397a81a6adfddd2fc3cd20` passed CI `34671218918` and Fuzz Smoke `34671218922`. No new parser was introduced, so no fuzz-target/workflow modification was justified.

## M6 Final Verification and Integration

- Documentation-inclusive PR head: `b429c3e853477b874d1f7b8ec366f7acfa1248e7`.
- Exact-head CI `34671648182` passed locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all workspace tests.
- Exact-head Fuzz Smoke `34671648146` passed the bounded protocol parser suite.
- Final PR review found 15 changed files, all limited to M6 policy/core/simulator/tests/docs scope; there were no submitted reviews or unresolved review threads.
- PR #16 was marked ready only after exact-head CI/fuzz success and merged with expected-head protection.
- Merge commit: `4d73528329fdae5d3eb1c25279778973168c87bd`.
- Merge tree: `a5992db1c6ffd6f1b7335f465fdd76dd48943880`, identical to the verified PR-head tree.
- Post-merge canonical `main` CI `34671715166` passed lockfile verification, rustfmt, workspace check, Clippy, and all tests.
- Fuzz Smoke is PR-triggered here; canonical merge-tree equivalence to the exact fuzz-verified PR tree supplies the M6 merge fuzz evidence without inventing a push fuzz run.

## Exact Next Task

Begin **M7 — disconnect/reconnect/revocation/failure simulator** from the verified canonical `main` state only after this documentation-only integration record is itself CI-verified.

M7 startup procedure:

1. inspect canonical `main`, recent commits, repository state, and this file;
2. read the uploaded Master Architecture plus M7-relevant sections of `docs/architecture/CORE-SIMULATOR.md`, `SESSION-TRANSPORT.md`, trust/revocation architecture, and relevant ADRs;
3. inspect the current M5/M6 session, control, stream, policy, trust, and memory-transport code before designing failure behavior;
4. inspect relevant uploaded research repositories for reconnect/cancellation/revocation/failure semantics before implementation;
5. write or confirm the smallest M7 design/implementation plan, preserve current architecture boundaries, and then execute test-first on a fresh M7 branch from the exact verified `main` head.

Do not silently extend M7 into real networking, persistence, UI/platform, or unrelated capability work.

## Resume Procedure

1. inspect `main`, active feature branches, recent commits, PR/workflow state, and repository cleanliness;
2. read the Master Architecture, this file, the active milestone plan, and relevant ADRs/architecture contracts;
3. reconcile docs with actual code before editing;
4. work in small verified milestones and merge only exact verified heads to `main`;
5. never rely on chat history or stash as the only copy of incomplete work.
