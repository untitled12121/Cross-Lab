# Phase 2 Resumable File Transfer

**Status:** Active — Task 1 implemented; ADR-0020 accepted; PR #65-#69 shared runtime complete; PR #70-#73 Linux storage/receive/service/product flow merged; Task 5 Android platform adapter active
**Date:** 2026-09-26
**Base:** PR #61 merged as `61ae275356201245a7dcac95d11bd598d2c3045e`  
**Foundation:** PR #62 merged as `e338d10911ccdb908bc495150f048503086c0ce2`; exact implementation head `e663909443d208c712ccb7f6e1e74fb8dab26ab8` passed full Rust + Android CI `36258390585`.

## Goal

Deliver explicit resumable Linux ↔ Android file transfer without weakening Cross-Lab's authenticated-session, exact-policy, data-plane authorization, reconnect, filesystem, or privacy boundaries.

## Existing baseline

The repository already provides:

- authenticated trusted sessions over Quinn;
- exact per-device capability policy and durable persist-before-apply edits;
- `files.transfer` with exact `send` / `receive` policy names;
- `AuthorizedOperation`, `OperationId`, and bounded `StreamAdmission`;
- bounded `DataStreamOpenV1` and real Quinn unidirectional data streams;
- stale-operation rejection across reconnect/revocation/policy changes;
- Linux GPUI and Android Compose product/runtime boundaries.

Phase 1 intentionally did not define real filesystem transfer or resume semantics.

## Research result

The reference review supports a deliberately smaller Cross-Lab design:

- Flying Carpet reinforces explicit cross-platform transfer, strict metadata bounds, and path sanitization, while reconnect resume remains outside its current shipped design.
- RustDesk demonstrates practical offset continuation and transfer-job lifecycle.
- Syncthing demonstrates block offset/hash verification, but its sync/version-vector/dedup model is beyond the Cross-Lab explicit-transfer MVP.

No reference architecture is copied. Cross-Lab keeps its own session, policy, authorization, and platform boundaries.

## Task 1 — Product data-stream runtime foundation — Implemented

Implement protocol-neutral runtime plumbing only:

- move reusable admitted-stream handling out of simulator-only ownership;
- keep one authenticated session as the authority source for control and data;
- expose bounded operation registration/open/accept/read/cancel primitives without file semantics;
- preserve `StreamAdmission` checks and local trust/policy revision validation;
- cancel all stream authority on policy replacement, reconnect, revocation, transport loss, and shutdown;
- add event-driven Quinn incoming-stream readiness rather than polling;
- cover backpressure, stale-session rejection, wrong operation binding, and lifecycle cancellation.

This task must not define file metadata, resume offsets, filesystem paths, or new wire semantics.

Implemented on PR #62. The shared runtime now owns authorized stream lifecycle, Quinn provides event-driven incoming-stream readiness, stale authority is cancelled on policy/revocation/transport/session shutdown paths, closed outbound streams release bounded runtime capacity, and stream payload debug output remains redacted. Exact implementation head `e663909443d208c712ccb7f6e1e74fb8dab26ab8` passed full Rust + Android CI `36258390585`.

## Task 2 — File-transfer v2 profile decision — Accepted

Owner accepted ADR-0020 as revised on 2026-09-26. The compatibility surface now fixes local approval-before-authority, `Ready` / `AlreadyComplete` acceptance, durable fail-closed checkpoint recovery, fresh session-bound `OperationId` authority on every attempt, bounded completion tombstones, and terminal `files.transfer.result` outcomes.

The proposal scopes v2.0 to explicit single-file push with:

- `files.transfer/receive`;
- bounded basename/size/BLAKE3 metadata;
- a 256-bit transfer correlation ID;
- 1 MiB contiguous resume checkpoints;
- a fresh session-bound `OperationId` for every transfer attempt/reconnect;
- one authorized source-to-destination data stream;
- partial-file state that survives reconnect without carrying session authority;
- bounded completion tombstones so a lost terminal result cannot duplicate an already completed transfer;
- a session-local `files.transfer.result` event for confirmed terminal outcome.

Any incompatible change to ADR-0020 now requires a new capability profile/version and architecture review.

## Task 3 — Shared transfer capability runtime — Implemented

PR #65 implemented the accepted bounded offer/accept/result payload contract and focused protocol specification. PR #66 connected Quinn's bounded stream-ready signal to `RuntimeActor`. PR #67 completed exact v2 capability registration, bounded offer/accept correlation, result subscription before the offer, default-disabled product availability, and fresh destination `SingleStream` authority after current exact authorization, with one hard capacity budget for pending/unused authority. PR #68 completed actor-owned open/send/finish/cancel stream execution, ownership-preserving backpressure, destination stream correlation, terminal-result handling, and Quinn-backed source-to-destination runtime coverage. PR #69 completed streaming BLAKE3 preparation/verification plus bounded retained partial/checkpoint/tombstone capability state.

After ADR-0020 acceptance:

- implement exact bounded offer/accept codecs — implemented on PR #65;
- issue/register the destination `AuthorizedOperation` only after exact authorization — implemented on PR #67;
- correlate bounded transfer state — control correlation implemented on PR #67; stream/data-plane correlation implemented on PR #68;
- stream source hashing and payload with bounded buffers/backpressure — actor/data-plane streaming implemented on PR #68; BLAKE3 integrity state implemented on PR #69;
- verify final size/digest — shared verifier implemented on PR #69;
- persist bounded partial-transfer metadata and verified checkpoint state — shared retained-state codec/model implemented on PR #69; platform persistence remains Tasks 4/5;
- expire abandoned partial transfers — bounded count/age pruning model implemented on PR #69; platform cleanup policy remains Tasks 4/5;
- never retain payload bytes in logs/history.

## Task 4 — Linux platform adapter — Implemented

PR #70 completed the first platform slice with a private atomic retained-state store, opaque/redacted Linux path locators, symlink-safe source opening checks, full streaming BLAKE3 preparation, and bounded 64 KiB resume reads. PR #71 completed private adjacent receive partials, fsync-before-durable 1 MiB checkpoint advancement, restart truncation/fail-closed recovery, exact whole-file BLAKE3 verification, completion tombstones, crash recovery across publication, and atomic non-clobber publication. PR #72 merged authenticated source binding plus the bounded Linux worker/service that consumes both agent queues, restores retained transfer state, drives receive/terminal handling, and exposes owner destination approval as a typed boundary. PR #73 completed native source/save prompts, bounded sender hashing/streaming, stable retry identity, progress/cancel/retry presentation, capability advertisement only with the complete product path, bounded age-based cleanup of partial/completed retained state, and the Ready-cancellation control-path repair. Its current cancellation checkpoint explicitly releases pending source offers, reports typed destination request/transfer cancellation through a bounded product channel, consumes already-issued stream authority when acceptance races cancellation, correlates save prompts by exact request ID, releases accepted platform receivers on session/policy/disconnect/operation-expiry cleanup while preserving retained resume state, adds an explicit owner decline path that returns typed `Cancelled` without minting stream authority or storage state, makes destination cancellation valid from `Ready` through active streaming and the pre-publication finished-stream window with exact operation/stream revocation and retained resume state, suppresses already-buffered events for cancelled stream identities, preserves source terminal correlation across transport-close/result ordering, keeps integrity/storage aborts on a separate atomic stream-failure path, avoids stale prompt resurrection after session loss, binds same-`TransferId` sender retry to the exact original offer identity so a changed local source cannot mutate retained transfer identity, drops path-bearing send state after terminal completion/source invalidation, and introduces a non-cancellable finalizing state once payload transmission has finished.

- explicit native file selection/save destination;
- no peer-supplied path authority;
- bounded streaming file I/O;
- private partial file + crash-safe checkpoint metadata;
- atomic/non-clobbering finalization where platform semantics permit;
- clean cancellation and restart behavior.

## Task 5 — Android platform adapter — Active

Active branch `phase2-android-file-transfer-adapter` starts from merged PR #73 / `main` `881a5dc0f50943d2cc312a5396afecc07c6a1073`.

The adapter slice keeps capability/UI authority separate while establishing the reusable mobile boundary:

- expose narrow UniFFI file-transfer request/cancellation/data/operation objects plus shared streaming BLAKE3 and retained-state helpers;
- use Storage Access Framework / ContentResolver boundaries for owner-selected source and destination documents;
- keep content URIs local to Android and redacted from normal debug output;
- retain URI permission only after explicit owner grant where required;
- keep resumable partial bytes app-private and no-backup, with shared source-bound retained metadata and 1 MiB durable checkpoints;
- stream source hashing, source reads, receive writes, verification, and final publication with bounded 64 KiB buffers;
- truncate bytes beyond the last durable checkpoint after lifecycle restart and fail closed when a retained partial is shorter than its checkpoint;
- keep Android production `files.transfer` advertisement disabled until Task 6 wires the complete owner-selection/progress/cancel/retry Compose path;
- require no broad storage privilege for the transfer feature.

## Task 6 — Product UI

Expose the smallest clear controls:

- explicit Send file action;
- receive/resume state and destination ownership;
- progress without file-content preview;
- pause/cancel/retry/resume states;
- permission/unavailable/storage/integrity failure states;
- no remote filesystem browser in v2.0.

## Verification

Use a two-level loop so development stays fast without weakening the merge gate.

During iteration, prefer long coherent coding sessions and batch all related implementation, regression tests, cleanup, and documentation into a meaningful milestone before pushing. Run only the checks affected by the change: formatting plus targeted crate/package checks, clippy/tests, desktop build, UniFFI generation, or Android tests/assembly as applicable. Do not rerun the complete workspace/mobile matrix after every small edit, and do not create a push for every small correction unless a durable interruption checkpoint is necessary.

Before marking a PR ready or merging, the exact head must pass the full gate:

- `cargo fmt --check`;
- `cargo check --workspace --all-targets --all-features`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo test --workspace --all-features`;
- Linux desktop normal + development builds;
- UniFFI generation;
- Android JVM tests + normal/development assemblies.

File-transfer-specific regression coverage must include:

- metadata and filename bounds/path traversal;
- exact default-deny / wrong operation / wrong peer;
- stale `OperationId` rejection after reconnect;
- policy/revocation cancellation;
- explicit pre-accept cancellation releases the pending request/authority and permits immediate same-`TransferId` retry;
- cancellation racing `Ready` consumes the fresh single-stream authority without sending payload bytes;
- destination save-prompt cancellation is correlated by exact request ID so a retry cannot inherit stale UI authority;
- explicit owner decline returns typed cancellation, creates no receiver/stream authority, and permits immediate same-`TransferId` retry;
- destination cancellation works immediately after `Ready`, during streaming, and after stream finish but before platform publication; it revokes exact authority, suppresses stale queued data events, and surfaces terminal `Cancelled` without publishing the file;
- session/policy/disconnect/operation-expiry interruption releases accepted platform state while preserving retained partial resume state;
- bounded data-channel failure releases platform active state instead of poisoning later retry with stale `AlreadyActive`;
- interrupted checkpoint recovery;
- same-`TransferId` retry preserves the exact original offer identity; a changed local source fails locally and requires a fresh owner selection/new transfer identity;
- changed source identity rejection;
- final size/digest mismatch;
- destination collision/non-clobber behavior;
- bounded memory/queues and backpressure.

## Scope boundary

Do not add folder sync, deduplication, content-defined chunking, remote filesystem browsing, background broad storage access, cloud storage dependency, or Phase 3 adaptive networking in this slice.
