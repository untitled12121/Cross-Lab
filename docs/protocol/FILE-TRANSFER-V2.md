# File Transfer Capability Profile v2

**Status:** Accepted implementation specification  
**Architecture:** ADR-0020, Master Architecture revision 2.10  
**Capability:** `files.transfer` v2.0

## 1. Scope

This specification fixes the first exact payload encoding for ADR-0020. It does not change the accepted architecture.

v2.0 is explicit single-file push using protected operation `receive`. Structured capability payloads use Protocol Buffers inside the existing control request/response/event body. They are not wrapped in another Cross-Lab frame because the containing control envelope already provides bounded framing and session sequencing.

## 2. Common rules

- profile field value: `2`;
- `TransferId`: exactly 32 random bytes;
- BLAKE3 digest: exactly 32 bytes;
- `OperationId`: exactly 32 bytes;
- display filename: non-empty UTF-8, at most 255 bytes, not `.` or `..`, and contains no `/`, `\\`, or NUL;
- unknown semantic enum values fail closed;
- payloads are size-checked before protobuf decoding;
- protobuf unknown fields retain normal forward-compatibility behavior but do not create authorization semantics.

Wire body limits:

| Body | Maximum encoded bytes |
|---|---:|
| Offer | 384 |
| Acceptance | 128 |
| Terminal result | 64 |

## 3. Offer

The source sends a `ControlRequest` with:

- capability `files.transfer`;
- capability version 2.0;
- operation `receive`;
- retry class `NonRetryable`;
- body `FileTransferOfferV2`.

```proto
message FileTransferOfferV2 {
  uint32 profile_version = 1; // exactly 2
  bytes transfer_id = 2;      // 32 bytes
  string display_name = 3;    // bounded basename
  uint64 file_size = 4;
  bytes blake3_digest = 5;    // 32 bytes
}
```

The display name is presentation metadata only and never a destination path.

## 4. Acceptance

A successful control response contains `FileTransferAcceptanceV2`.

```proto
message FileTransferReadyV2 {
  bytes transfer_id = 1;
  uint64 resume_offset = 2;
  bytes operation_id = 3;
}

message FileTransferAlreadyCompleteV2 {
  bytes transfer_id = 1;
}

message FileTransferAcceptanceV2 {
  uint32 profile_version = 1; // exactly 2
  oneof result {
    FileTransferReadyV2 ready = 2;
    FileTransferAlreadyCompleteV2 already_complete = 3;
  }
}
```

`Ready` is valid only for the exact offer being processed. The destination validates `resume_offset` against the offer: it must not exceed `file_size`, and must be either the exact file size or a multiple of 1,048,576 bytes.

`AlreadyComplete` never contains an `OperationId`.

Capability-level rejection after dispatch uses the existing typed control-response error path. Shared session/policy rejection continues to use the existing dispatcher behavior.

## 5. Data stream

For `Ready`, the source opens one existing `DataStreamOpenV1` with:

- the current authenticated `SessionId`;
- returned `OperationId`;
- capability `files.transfer` version 2.0;
- operation `receive`;
- direction `SourceToDestination`;
- stream index 0.

The stream carries exactly `[resume_offset, file_size)` bytes.

No file bytes are valid before normal stream admission succeeds.

## 6. Terminal result

Before sending the offer, the source subscribes to capability event type `files.transfer.result`.

The event body is:

```proto
enum FileTransferTerminalOutcomeV2 {
  FILE_TRANSFER_TERMINAL_OUTCOME_UNSPECIFIED = 0;
  FILE_TRANSFER_TERMINAL_OUTCOME_COMPLETED = 1;
  FILE_TRANSFER_TERMINAL_OUTCOME_CANCELLED = 2;
  FILE_TRANSFER_TERMINAL_OUTCOME_INTEGRITY_FAILED = 3;
  FILE_TRANSFER_TERMINAL_OUTCOME_STORAGE_FAILED = 4;
}

message FileTransferResultV2 {
  uint32 profile_version = 1; // exactly 2
  bytes transfer_id = 2;      // 32 bytes
  FileTransferTerminalOutcomeV2 outcome = 3;
}
```

`UNSPECIFIED` and unknown outcomes are invalid.

The result is correlation/status only. It grants no authority.

## 7. Resume and retry

The fixed durable checkpoint size is 1 MiB (1,048,576 bytes).

After reconnect:

- old session/request/stream/`OperationId` authority is invalid;
- the source repeats the same offer identity tuple;
- an incomplete destination returns `Ready` with its last durable checkpoint plus a fresh `OperationId`;
- a retained completed destination returns `AlreadyComplete`.

Exact retention/cleanup duration is local policy and not wire compatibility.

## 8. Privacy

Debug output for the offer must not include the display filename or digest. Normal logs/audit must not include file contents or platform paths.
