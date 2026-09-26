# ADR-0020: Resumable single-file transfer profile v2

**Status:** Proposed
**Date:** 2026-09-26

## Context

Phase 2 requires resumable file transfer between Linux and Android after the text clipboard slice.

The existing architecture already reserves `files.transfer`, exact protected operations `send` and `receive`, authenticated control requests, `AuthorizedOperation`, `OperationId`, `DataStreamOpenV1`, bounded unidirectional data streams, and reconnect invalidation. Phase 1 deliberately used synthetic `files.transfer/send` bytes only to prove stream authorization; it did not define filesystem metadata, resumability, chunking, partial-file persistence, or product save semantics.

The Master Architecture reserves `files.transfer v2`, so Phase 2 needs an explicit capability profile before Linux/Android product code can depend on file-transfer payload semantics.

Reference review informed the design without copying source architecture:

- Flying Carpet demonstrates a small cross-platform explicit-transfer product and reinforces strict filename/header bounds, but its current release plan still treats reconnect resume as deferred.
- RustDesk demonstrates practical offset-based continuation and transfer-job lifecycle handling.
- Syncthing demonstrates stronger block offset/hash verification, but its synchronization, version-vector, deduplication, and folder-replication model is intentionally broader than Cross-Lab's explicit transfer MVP.

Foundation review also exposed three profile requirements that must be explicit before acceptance: local approval/destination selection must not mint stream authority early, the source needs a terminal result after the one-way data stream finishes, and retry after a lost terminal result must not duplicate a transfer that already completed.

## Decision

If accepted, the first Cross-Lab file-transfer product profile is `files.transfer` capability version **2.0** and provides explicit, single-file, resumable push transfer.

### Product operation

The Phase 2 v2.0 product path uses:

- CapabilityId: `files.transfer`
- OperationName: `receive`
- RetryClass: `NonRetryable`

A source device sends a `receive` control request to the destination device. The operation name describes the protected action performed by the request receiver: accepting file bytes from the authenticated peer.

An `Ask` policy decision does not authorize the transfer. No successful acceptance or `OperationId` may be issued until existing local approval requirements are satisfied and policy reevaluation produces `Allow`. Peer-supplied state and owner selection of a save location are not substitutes for policy approval evidence. v2.0 does not introduce a new remote approval mechanism.

The existing `files.transfer/send` policy operation remains reserved but is not part of the v2.0 product path. A future remote-pull design must not expose arbitrary local filesystem paths and requires a separately versioned compatibility decision.

### Transfer offer

The request body is a bounded versioned transfer offer containing:

- a random 256-bit `TransferId` used only for transfer correlation;
- a UTF-8 display filename containing one basename only;
- exact file size as `u64`;
- a BLAKE3-256 digest of the complete source file.

Bounds:

- filename is at most 255 UTF-8 bytes;
- filename must not contain path separators, NUL, `.`, or `..` path components;
- one request describes exactly one regular file;
- directories, symlinks, device nodes, sparse-file semantics, extended attributes, and alternate streams are not represented by v2.0;
- platform/policy storage limits may reject a file before transfer even when the wire representation is valid.

`TransferId` is correlation metadata, not bearer authority. Possessing or guessing it never replaces authenticated session, exact policy, or `OperationId` checks.

### Acceptance response and operation authority

A successful destination response is a bounded versioned acceptance result with one of two states:

- `Ready`: the same `TransferId`, a destination-validated `resume_offset`, and a fresh `OperationId` authorizing one source-to-destination data stream;
- `AlreadyComplete`: the same `TransferId` and no `OperationId`, used only when bounded retained completion state proves that the same authenticated source and exact file identity tuple already completed.

Rejected, cancelled, unavailable, or resource-limited offers use the existing typed control failure path and never carry an `OperationId`.

Before returning `Ready`, the destination:

1. verifies negotiated `files.transfer` v2.0 and exact local `receive` policy, including any required local approval;
2. resolves owner-controlled destination/partial storage and validates or creates bounded partial-transfer state for the offered file identity;
3. issues and locally registers an `AuthorizedOperation` bound to the current authenticated session, peer, capability/version, `receive`, current trust revision, and current policy revision;
4. uses `UsePolicy::SingleStream`.

The source opens one existing `DataStreamOpenV1` with:

- the current `SessionId`;
- the returned `OperationId`;
- `files.transfer` v2.0;
- operation `receive`;
- direction `SourceToDestination`;
- stream index 0.

No data bytes are delivered to file-transfer code until existing stream admission succeeds.

### Resume model

v2.0 resumes only from the destination's **largest durable contiguous checkpoint**.

The fixed resume checkpoint size is **1 MiB (1,048,576 bytes)**. Except for the complete file length, a nonzero `resume_offset` must be aligned to this checkpoint size.

The destination may persist bounded transfer state needed to recover a partial transfer:

- `TransferId`;
- source `DeviceId`;
- sanitized display filename;
- expected size;
- expected BLAKE3-256 digest;
- durable contiguous offset;
- platform-local partial-file locator.

The platform-local locator is never sent over the network and is not identity or authorization material. On restart, the implementation must validate that persisted metadata and the partial file are mutually consistent, truncate or ignore bytes beyond the durable offset, and never advance `resume_offset` merely because a longer partial file exists. Ambiguous or inconsistent state rolls back to an earlier valid checkpoint or fails closed.

After interruption, the source repeats the transfer offer using the same `TransferId`, size, name, and digest. The destination returns the last durable checkpoint. A changed identity tuple is rejected rather than merged with the previous partial file.

After successful final publication, the destination retains a bounded completion tombstone containing the `TransferId`, authenticated source identity, and exact file identity tuple but no active operation/session authority. A matching repeated offer may return `AlreadyComplete`; a changed tuple is rejected. Completion tombstones and partial state are subject to bounded local retention/cleanup policy.

A resumed authenticated session always receives a **new `OperationId`**. Old `SessionId`, `OperationId`, stream, request sequence, or transport authority never survives reconnect.

### Data and integrity

The data stream carries exactly the bytes in the half-open range:

`[resume_offset, file_size)`

The capability layer streams with bounded buffers and backpressure; it does not load the complete file into memory.

The destination:

- writes only to a partial/temporary target;
- advances the durable resume offset only after complete checkpoint bytes are committed;
- truncates or ignores an incomplete trailing checkpoint after interruption;
- verifies exact final length and BLAKE3-256 digest before publishing the completed file;
- resets or discards untrusted partial state after a final digest mismatch rather than resuming from bytes that failed whole-file integrity;
- never silently replaces an unrelated existing destination file.

QUIC/TLS integrity protects transport records, while the file digest binds resume state and final content identity across separately authenticated sessions.

### Terminal result

The source subscribes on the current authenticated session to capability event type `files.transfer.result` before sending the offer. The destination emits one bounded versioned terminal-result event containing:

- the `TransferId`;
- one outcome: `Completed`, `Cancelled`, `IntegrityFailed`, or `StorageFailed`.

The event is status/correlation only and carries no authority. `Completed` is emitted only after exact final length/digest verification, non-clobbering publication, and durable completion-tombstone update. The source treats duplicate terminal results for the same `TransferId` idempotently.

A terminal result can be lost with the session. Loss does not create ambiguity or duplicate authority: after reconnect the source subscribes again and repeats the same offer. The destination then returns `AlreadyComplete` for a retained completed transfer or `Ready` with its last durable checkpoint for an incomplete transfer.

v2.0 does not require wire-level progress events; each side may derive local progress from bounded byte counts.

### Lifecycle

Pending transfer authority is cancelled on:

- cancellation of an offer before acceptance;
- policy revision replacement;
- peer revocation;
- disconnect or transport loss;
- session replacement/reconnect;
- explicit transfer cancellation;
- shutdown;
- operation expiry.

Partial file metadata or a bounded completion tombstone may remain for later retry/idempotency, but retained transfer state carries no active session authority. Event subscriptions and `OperationId` values are session-local and are recreated after reconnect.

### Platform boundary

Linux and Android own filesystem/UI access outside shared protocol/runtime code.

Linux:

- source selection and destination selection stay in the desktop platform/UI boundary;
- shared Rust receives bounded streaming handles/adapters, not arbitrary UI-originated authority.

Android:

- source/destination access uses platform document/content APIs and persisted URI permission only when explicitly granted;
- content URIs and filesystem paths are never protocol identifiers;
- no broad storage privilege is required solely for Cross-Lab transfer.

Both platforms sanitize peer-provided display names and keep final destination choice owner-controlled.

### Privacy

Normal logs/audit must not contain file contents or platform paths.

Audit may contain bounded non-content metadata already permitted by the privacy architecture, such as peer `DeviceId`, capability/operation, `TransferId`, byte count, result, and whether a transfer resumed.

## Alternatives considered

### Reuse a session-scoped OperationId across reconnect

Rejected. It violates the existing fresh-session authority model and creates cross-session replay authority.

### Arbitrary block-map resume and deduplication

Deferred. Syncthing-style block availability and deduplication are valuable for synchronization but add manifest size, state, scheduling, and persistence complexity unnecessary for explicit MVP transfer.

### Resume from any byte offset

Rejected for v2.0. Durable fixed checkpoints bound recovery state and avoid claiming progress for partially committed data.

### Multi-file/folder bundle in the first profile

Deferred. One regular file keeps filename/path handling, destination ownership, progress, cancellation, and atomic completion small enough to verify before adding bundle semantics.

### Put file bytes in control messages

Rejected. Bulk payloads remain on the authorized data plane and must not pressure the control queue.

### Trust filename/remote path as the destination path

Rejected. Peer metadata never grants filesystem authority.

## Security impact

The profile preserves the existing rule that control authorizes and data carries payload. `TransferId`, filename, stream ID, transport identity, and `OperationId` are never sufficient authority independently.

Resume state is bound to authenticated peer identity and complete-file digest, while every resumed stream receives fresh session-scoped authorization. Path traversal, stale-operation replay, policy/trust revision changes, oversized metadata, wrong transfer identity, and final digest mismatch fail closed.

## Compatibility impact

If accepted, the following become the `files.transfer` v2.0 compatibility surface:

- explicit single-file push via operation `receive`;
- versioned bounded offer and `Ready` / `AlreadyComplete` acceptance bodies;
- 256-bit `TransferId`;
- UTF-8 basename and size/digest metadata;
- BLAKE3-256 complete-file identity;
- 1 MiB durable contiguous resume checkpoints;
- bounded completion tombstones for idempotent retry after a lost terminal result;
- capability event `files.transfer.result` with terminal `Completed`, `Cancelled`, `IntegrityFailed`, or `StorageFailed` outcome;
- fresh `OperationId` and existing `DataStreamOpenV1` for each authenticated session;
- source-to-destination raw byte range beginning at `resume_offset`.

Incompatible changes require a new capability minor/major profile rather than silent reinterpretation.

## Operational impact

Initial send performs a source-file BLAKE3 pass before transfer so resume identity is known before bytes are accepted. This trades extra source I/O for simple, strong cross-session resume correctness. Implementations should stream hashing with bounded buffers and surface a preparing state rather than blocking UI.

Partial-transfer persistence and completion tombstones are capability state, separate from identity/policy stores, and must have bounded cleanup/expiry rules before product rollout. Exact local retention duration is not wire compatibility.

## Consequences

Cross-Lab gets a small resumable transfer model that reuses existing authenticated control, capability events, and authorized data-stream boundaries instead of creating a second transfer transport. The terminal-result event plus bounded completion tombstone gives the source a confirmed completion path without turning a transfer identifier into authority or duplicating a completed transfer after acknowledgement loss.

The proposal intentionally does not authorize implementation of v2 payload codecs until accepted. Protocol-neutral stream runtime plumbing may proceed independently because it preserves already-accepted `OperationId` and `DataStreamOpenV1` semantics.
