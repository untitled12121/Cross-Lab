# ADR-0018: Text clipboard capability profile v1

**Status:** Accepted
**Date:** 2026-09-24  
**Accepted:** 2026-09-25

## Context

Phase 2 requires clipboard interoperability between Linux and Android on the authenticated Cross-Lab session/policy foundation.

The existing architecture already names clipboard.read and clipboard.write, and the Phase 1 simulator uses the exact protected operations get and set at capability version 1.0. The generic protocol intentionally leaves capability request/response bodies opaque, so production clipboard payload semantics and bounds must be defined before interoperability code depends on them.

Android does not provide a reliable normal-app background clipboard-read contract. Cross-Lab therefore cannot make continuous background clipboard monitoring a required v1 behavior.

Clipboard contents are sensitive payloads and are prohibited from normal logs/audit history.

## Decision

If accepted, clipboard capability profile v1 uses protocol capability version 1.0 and explicit request/response operations.

### Write remote clipboard

CapabilityId: clipboard.write
OperationName: set
RetryClass: NonRetryable
Request body: UTF-8 clipboard text
Success body: empty

The request body:

- is valid UTF-8;
- is at most 65,536 bytes;
- may be empty, which represents setting/clearing text clipboard content;
- contains text only.

The receiver applies the text through its local platform adapter before returning success. Platform failure returns a typed protocol failure without clipboard content in diagnostics.

### Read remote clipboard

CapabilityId: clipboard.read
OperationName: get
RetryClass: NonRetryable
Request body: empty
Success body: UTF-8 clipboard text

The success body follows the same 65,536-byte UTF-8 limit. A non-empty request body is invalid for this operation.

A platform may report the operation unavailable when local OS privacy/focus rules prevent reading the clipboard.

### No automatic clipboard event in v1

The product v1 clipboard path does not use clipboard.changed for continuous synchronization.

A device sends or fetches clipboard text only after an explicit product action or a later separately approved auto-share policy. This avoids making background clipboard observation a compatibility requirement and avoids feedback-loop semantics in the first product capability.

### Content scope

Clipboard v1 does not carry:

- files;
- images;
- rich-text/HTML representations;
- password-manager metadata;
- multiple MIME representations.

Large/binary clipboard objects belong to the file-transfer/data-stream capability rather than ordinary control frames.

### Privacy

Plaintext clipboard contents:

- are never written to normal logs, audit history, diagnostics, or Debug output;
- are held only for the bounded operation lifetime needed to transfer/apply them;
- are not retained as Cross-Lab clipboard history by this profile.

Audit may record non-sensitive metadata already allowed by the privacy specification, such as capability/operation, peer device, result code, and bounded byte count.

## Alternatives considered

### Continuous clipboard.changed auto-sync as v1

Rejected for the initial profile. It creates Android background-access mismatch, feedback-loop rules, and more persistent observation than the MVP requires.

### Reuse one capability for both directions

Rejected. The existing architecture intentionally separates clipboard.read/get from clipboard.write/set, allowing the owner to authorize disclosure and mutation independently.

### Files/images inside clipboard control frames

Rejected. It bypasses the intended data-plane/file-transfer boundary and creates unnecessary control-plane memory pressure.

### Unbounded text up to the transport frame maximum

Rejected. Clipboard is capability-specific user data and should have a smaller explicit bound than the generic 256 KiB control frame maximum.

## Security impact

Every request remains subject to authenticated session identity, negotiated capability/version, exact local capability availability, and exact per-device policy evaluation.

Separating read/get and write/set prevents permission to mutate a clipboard from implicitly granting permission to exfiltrate the peer's current clipboard.

The explicit-action baseline minimizes passive clipboard observation. Sensitive payload redaction remains mandatory.

## Compatibility impact

If accepted, the following become the clipboard v1 compatibility surface:

- capability IDs clipboard.read and clipboard.write;
- operation names get and set;
- capability version 1.0;
- UTF-8 text payload interpretation;
- 65,536-byte text limit;
- empty-body rules described above;
- explicit request/response semantics.

Changing these semantics incompatibly requires a new clipboard capability version/profile.

## Operational impact

Linux can use the existing GPUI platform clipboard API without a new dependency.

Android clipboard reads are best-effort under current platform focus/privacy restrictions; the UI must surface unavailability instead of claiming background synchronization. Clipboard writes use the platform clipboard API.

## Consequences

Cross-Lab gets a small, interoperable, policy-separated text clipboard profile that works within the existing authenticated control plane and respects Android platform limits.

Automatic clipboard synchronization, richer MIME types, files/images, and clipboard history remain future work rather than hidden v1 behavior.
