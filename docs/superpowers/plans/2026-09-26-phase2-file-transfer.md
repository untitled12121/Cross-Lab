# Phase 2 Resumable File Transfer

**Status:** Active — Task 1 implemented; ADR-0020 accepted; Task 3 ready
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

## Task 3 — Shared transfer capability runtime — Active

After ADR-0020 acceptance:

- implement exact bounded offer/accept codecs;
- issue/register the destination `AuthorizedOperation` only after exact authorization;
- correlate bounded transfer state;
- stream source hashing and payload with bounded buffers/backpressure;
- verify final size/digest;
- persist bounded partial-transfer metadata and verified checkpoint state;
- expire abandoned partial transfers;
- never retain payload bytes in logs/history.

## Task 4 — Linux platform adapter

- explicit native file selection/save destination;
- no peer-supplied path authority;
- bounded streaming file I/O;
- private partial file + crash-safe checkpoint metadata;
- atomic/non-clobbering finalization where platform semantics permit;
- clean cancellation and restart behavior.

## Task 5 — Android platform adapter

- use Storage Access Framework / ContentResolver boundaries;
- retain URI permission only after explicit owner grant where required;
- keep content URIs local to Android;
- bounded streaming I/O without whole-file buffering;
- partial-state recovery compatible with app lifecycle;
- no broad storage privilege for the transfer feature.

## Task 6 — Product UI

Expose the smallest clear controls:

- explicit Send file action;
- receive/resume state and destination ownership;
- progress without file-content preview;
- pause/cancel/retry/resume states;
- permission/unavailable/storage/integrity failure states;
- no remote filesystem browser in v2.0.

## Verification

For every implementation milestone:

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
- interrupted checkpoint recovery;
- changed source identity rejection;
- final size/digest mismatch;
- destination collision/non-clobber behavior;
- bounded memory/queues and backpressure.

## Scope boundary

Do not add folder sync, deduplication, content-defined chunking, remote filesystem browsing, background broad storage access, cloud storage dependency, or Phase 3 adaptive networking in this slice.
