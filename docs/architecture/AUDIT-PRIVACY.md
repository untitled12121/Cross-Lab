# Cross-Lab Audit and Privacy Rules

**Status:** Phase 0 specification  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0

## 1. Purpose

Cross-Lab requires enough auditability to explain sensitive actions without turning logs into a second copy of private user data or credentials. Audit events are security metadata and must be minimized, structured, and access-controlled when persistence is introduced.

## 2. Event classes

Security-sensitive events include at least:

- pairing invitation/start/success/failure/cancellation;
- trust establishment, credential rotation, revocation;
- authentication/session establishment/failure/close;
- protected policy allow/deny/ask decisions where useful for accountability;
- privileged-operation requests/results;
- recovery commands/results;
- update verification/install results;
- security parsing/replay/resource-limit failures when recording them is useful and safe.

High-volume data-plane progress is telemetry, not automatically a permanent audit record.

## 3. Allowed normal audit fields

Where applicable:

```text
event type / schema version
timestamp from local system
local DeviceId
source/destination DeviceId
public KeyId/fingerprint
CapabilityId / OperationName
OperationId or transition/command/request identifier
policy/trust revision
transport class (not raw secret metadata)
authorization/result/error code
non-sensitive duration/count metrics
```

Fields are included only when needed for the event's accountability/diagnostic purpose.

## 4. Prohibited normal logging/audit content

Never include by default:

- private keys or seed material;
- recovery private keys/secrets;
- pairing bootstrap secrets;
- reusable authentication credentials/tokens/passwords;
- raw session secrets or transport key material;
- plaintext clipboard contents;
- file contents;
- camera/microphone/screen/audio payloads;
- raw recovered data;
- unrestricted command output containing user data;
- full protocol frames merely for convenience.

Debug builds do not automatically waive these rules.

## 5. Sensitive metadata

Device topology, presence, network addresses, file names/paths, application names, notification metadata, authentication attempts, and recovery/location information can be sensitive even when not payload contents.

Such fields require a concrete diagnostic/user-facing purpose and should be redacted, hashed, summarized, or omitted when the purpose can be met with less information.

## 6. Diagnostics

Peer-visible protocol diagnostics are capped and safe. Internal error details that reveal secrets, filesystem paths, key material, policy internals, or captured payloads are not automatically forwarded to remote peers.

User-visible diagnostics should explain actionable failures without exposing security material.

## 7. Persistence

Phase 1 simulator audit sinks are in-memory test structures. Persistent audit storage is deferred until durable state is introduced.

Before persistent audit logs ship, the storage design must define:

- owner/user access controls;
- retention/deletion policy;
- file/database permissions and at-rest protection appropriate to the platform;
- rotation/size limits;
- export/redaction behavior;
- corruption/failure behavior;
- whether any field requires additional encryption.

A persistence technology is not selected by this document.

## 8. Availability/resource controls

Logging must be bounded. A malicious peer must not create unlimited log growth through repeated malformed requests or authentication failures. Implementations should rate-limit/coalesce repetitive events where this does not hide important security transitions.

Audit failure must not convert an authorization denial into an allow. For operations whose security policy requires durable audit before execution, failure to record must fail closed; such requirements are capability-specific and must be explicit.

## 9. Recovery/location privacy

Recovery actions, especially location, are visible/auditable and owner-authorized. Recovery must not create a hidden continuous location history by default. `LocateOnce` represents a bounded recovery request, not surveillance infrastructure.

## 10. Tests/review

Security-sensitive features must include review/tests ensuring secret-bearing types are not formatted through ordinary `Debug`/display/logging paths and that peer-visible errors do not contain sensitive payloads.

This document addresses the audit/privacy requirements of the Master Architecture and `TM-016`, with resource-safety interaction from `TM-017` and fail-closed behavior from `TM-022`.
