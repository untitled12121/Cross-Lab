# Cross-Lab Protocol v1

**Status:** Phase 0 specification  
**Milestone:** P0.6 — Protocol + Compatibility Model  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0

## 1. Purpose

This document defines the Phase 1 control-protocol encoding, compatibility negotiation, framing limits, message lifecycle, replay/cancellation/retry semantics, and canonical security transcript used for signatures/MACs.

Subject to `ADR-0004-protocol-encoding-v1.md`, Protocol Buffers is the ordinary v1 wire format. Protobuf bytes are never the canonical signature representation.

## 2. Version model

```text
ProtocolVersion
  major: u16
  minor: u16
```

A peer advertises one or more bounded support ranges:

```text
ProtocolRange
  major
  min_minor
  max_minor
```

Rules:

- maximum advertised ranges: 8;
- `min_minor <= max_minor`;
- choose the highest common major, then the highest minor in the overlapping range;
- no common major/minor -> `IncompatibleProtocol`;
- Phase 1 implements major `1` only;
- minor versions may add backward-compatible optional behavior but cannot reinterpret existing security semantics;
- a breaking field interpretation, security contract, or mandatory semantic change requires a new major or explicitly negotiated feature/profile.

## 3. Feature negotiation

Protocol hello may advertise bounded sets of numeric feature IDs:

```text
supported_features: max 64
required_features:  max 32
```

A peer's required feature must be present in the other peer's supported set. Otherwise session establishment fails.

Unknown optional feature IDs are ignored as unavailable. Unknown required feature IDs fail negotiation.

Phase 1 should define only features it actually implements rather than speculative feature IDs.

## 4. Protobuf schema rules

The `.proto` schema is the public wire-contract source of truth once implementation starts.

Rules:

- field numbers are never reused after release;
- removed fields are reserved in `.proto` schemas;
- `oneof` is used for mutually exclusive message bodies;
- missing/unknown envelope body is rejected;
- security-relevant enum values are validated explicitly from their numeric wire value;
- unknown enum/feature values are never mapped to an authorization-success default;
- security-domain `bytes` fields have exact validated lengths after decoding;
- strings used as domain identifiers pass their type-specific ASCII/length validator;
- unknown protobuf fields may be ignored only when negotiated compatibility says they are optional and they do not alter a signed security transcript.

## 5. Bootstrap/control framing

Every protobuf control message is carried in a length-delimited frame:

```text
u32_be payload_length
payload_length bytes protobuf payload
```

The length prefix itself is not protobuf encoded.

### 5.1 Limits

```text
bootstrap/hello frame maximum:  65,536 bytes
normal control frame maximum:   262,144 bytes
data-stream open header maximum: 4,096 bytes
capability entries per peer:    256
protocol support ranges:        8
supported feature IDs:          64
required feature IDs:           32
policy obligations in a wire decision/result: 32
safe diagnostic UTF-8 text:     512 bytes
```

Message-specific schemas may impose smaller limits.

A declared length above the applicable maximum is rejected before allocating the declared payload size. Truncated frames, invalid protobuf, invalid exact-length identifiers, oversized collections, and invalid domain strings are protocol errors.

Bulk user data is never embedded into ordinary control frames simply to bypass data-plane limits.

## 6. Session envelope

After secure-session establishment, every control-plane frame contains an envelope logically equivalent to:

```text
EnvelopeV1
  protocol_major
  protocol_minor
  session_id: 32 bytes
  message_seq: u64
  body: oneof
    capability_advertisement
    control_request
    control_response
    event
    cancel_request
    protocol_error
    session_close
```

Pairing/session-authentication bootstrap messages use dedicated pre-session message types and do not pretend to have an established `session_id`.

## 7. Session and message sequence

`SessionId` is a fresh 256-bit value bound to the secure-session transcript defined in P0.7.

Each direction of the ordered control channel maintains its own `message_seq`:

- first ordinary envelope uses sequence `0`;
- each next envelope increments by exactly one;
- duplicate/lower sequence -> replay/sequence error;
- gap on an ordered reliable control channel -> protocol/session error;
- reconnect creates a new `SessionId` and fresh directional sequence state;
- sequence values from an old session never authorize messages in a new session.

The sequence is an application-level anti-replay/state-integrity check in addition to transport protections.

## 8. Request identifiers

`RequestId` is a 128-bit cryptographically random identifier scoped to a logical session.

A `ControlRequestV1` contains:

```text
request_id
capability_id
capability_version
operation_name
retry_class
operation-specific request body/reference
```

A response contains the same `request_id` and either a typed success body or typed error result.

A request ID is correlation, not authorization. The receiver still authenticates the session and evaluates policy/operation semantics.

## 9. Retry and duplicate semantics

```text
RetryClass
  NonRetryable = 0
  Idempotent   = 1
```

Rules:

- generic protocol code never automatically retries `NonRetryable` requests;
- a duplicate `NonRetryable` `RequestId` in the same session is rejected as `DuplicateRequest`;
- an `Idempotent` request may be retransmitted with the same `RequestId` only when its capability operation declares the same semantics;
- receiver maintains a bounded per-session recent-request cache for idempotent requests and may return the recorded result rather than execute twice;
- the cache is bounded by count/time and disappears with the session;
- a reconnect never assumes an old `RequestId` is safe to re-execute unless a capability-specific cross-session idempotency design exists.

Phase 1 does not add a generic durable exactly-once protocol.

## 10. Timeouts

Request timeout is primarily a local caller/session policy. A sender timing out may issue cancellation and stop waiting.

Peer-provided wall-clock time is not trusted to extend authorization or disable local expiry. Security-critical local operation deadlines use local monotonic timing where practical.

## 11. Cancellation

`CancelRequestV1` references a `RequestId`.

Cancellation is idempotent:

- cancelling an unknown/already-complete request returns a typed harmless result or is ignored according to the request state machine;
- cancellation cannot create permission;
- cancelled operation/request resources are released promptly;
- cancelling a control request also cancels any not-yet-authorized operation creation associated with it;
- cancellation of an existing `AuthorizedOperation` uses its operation lifecycle in P0.5.

## 12. Events

Events are session-authenticated control messages and receive normal envelope sequence protection.

Each event contains:

```text
event_id: 128-bit random
capability_id / system event namespace
event_type: validated typed value
body
```

Events do not bypass policy merely because they are one-way. Event families define whether subscription/receipt requires prior authorization.

## 13. Capability advertisement

Phase 1 capability advertisement contains a bounded list of:

```text
CapabilityAdvertisementEntry
  capability_id
  min_version
  max_version
  runtime_available
  direction where defined
  bounded feature/limit metadata where defined
```

The advertisement travels only after peer authentication/session protection. It describes support; it grants no permission.

Negotiated capability state is derived from the intersection of local support and peer advertisement under P0.5/P0.7 rules.

## 14. Data stream open header

A protected data stream begins with a bounded `DataStreamOpenV1` header before raw/bulk payload bytes:

```text
DataStreamOpenV1
  session_id: 32 bytes
  stream_id: 16 random bytes
  operation_id: 32 bytes
  capability_id
  capability_version
  operation_name
  direction
  stream_index: u32
```

The header is framed with the 4 KiB maximum and validated against local `AuthorizedOperation` state before payload processing.

A correct `OperationId` with wrong source/session/capability/version/operation/direction/use index is rejected.

## 15. Session close

`SessionCloseV1` includes a typed close reason and optional safe diagnostic text. Receipt initiates graceful cancellation of active session-scoped work; local security events such as revocation may force close without waiting for peer acknowledgment.

## 16. Protocol errors

Wire errors use a stable numeric `ProtocolErrorCode` plus an optional safe diagnostic string capped at 512 UTF-8 bytes.

Phase 1 defines at least:

```text
MalformedFrame
FrameTooLarge
UnsupportedMessage
IncompatibleProtocol
UnsupportedRequiredFeature
InvalidIdentifier
InvalidSequence
ReplayDetected
DuplicateRequest
InvalidSession
AuthenticationFailed
TrustDenied
AuthorizationDenied
CapabilityUnsupported
CapabilityVersionIncompatible
OperationMissing
OperationExpired
OperationRevoked
OperationMismatch
Cancelled
ResourceLimit
InternalFailure
```

Diagnostics are not protocol semantics and must not include secrets/sensitive payloads.

## 17. Canonical transcript v1

Security-sensitive signatures/MACs operate on canonical Cross-Lab transcript bytes, never protobuf serialization output.

### 17.1 Canonical container

Exact byte layout:

```text
magic              17 bytes ASCII: "crosslab-canon-1\0"
domain_len         u16 big-endian
domain             domain_len bytes, validated ASCII
field_count        u16 big-endian
repeated fields:
  tag              u16 big-endian
  value_len        u32 big-endian
  value            value_len bytes
```

Rules:

- fields appear in strictly increasing numeric tag order;
- duplicate tags are invalid;
- tag `0` is reserved/invalid;
- omitted optional fields are absent, not encoded as empty unless the object's specification says empty is a valid distinct value;
- field count must equal the actual encoded field count;
- canonical encoding never depends on map iteration order;
- nested security objects are flattened by their object specification or represented by an explicitly defined digest field.

### 17.2 Canonical scalar values

```text
u16  -> exactly 2 bytes big-endian
u32  -> exactly 4 bytes big-endian
u64  -> exactly 8 bytes big-endian
bool -> one byte: 0x00 false, 0x01 true
OwnerId/DeviceId/KeyId/digest -> exactly 32 raw bytes
PairingId/RequestId/StreamId -> exactly 16 raw bytes
public keys -> canonical algorithm-specific raw bytes
ASCII domain identifiers -> already validated canonical bytes
```

Enums used in signed objects are encoded as `u16` using the stable registry values in this specification.

### 17.3 Transcript digest

```text
TranscriptDigestV1 = BLAKE3-256(
  "crosslab.transcript-digest.v1\0" || canonical_transcript_bytes
)
```

Ed25519 identity signatures sign exactly the 32-byte transcript digest for the specified object/profile.

Pairing HMAC confirmation uses the same pairing transcript digest with the distinct directional labels defined by P0.4.

### 17.4 Signed-object digest

When one signed object must be referenced inside another transcript without re-encoding it:

```text
SignedObjectDigestV1 = BLAKE3-256(
  "crosslab.signed-object-digest.v1\0" ||
  transcript_digest ||
  signature_algorithm_u16_be ||
  signature_bytes
)
```

The referenced object's schema defines whether this digest or only its unsigned transcript digest is required.

## 18. Canonical enum registry v1

Security transcript values defined so far:

```text
SignatureAlgorithm
  1 = Ed25519

AuthorityRole
  1 = OwnerRoot
  2 = DeviceSigning
  3 = Administrative
  4 = Recovery

TrustTransitionAction
  1 = Revoke
```

Values are never renumbered. New values are additive only when the relevant transcript/schema version permits them.

## 19. Canonical object domains and field tags

### 19.1 Authority delegation

Domain: `crosslab.authority-delegation.v1`

```text
1 schema_version: u16
2 owner_id: 32 bytes
3 role: u16
4 delegated_key_id: 32 bytes
5 delegated_algorithm: u16
6 delegated_public_key: bytes (Ed25519 v1 = 32 bytes)
7 delegation_epoch: u64
8 issuer_root_key_id: 32 bytes
```

### 19.2 Device credential

Domain: `crosslab.device-credential.v1`

```text
1 schema_version: u16
2 owner_id: 32 bytes
3 device_id: 32 bytes
4 device_key_id: 32 bytes
5 device_algorithm: u16
6 device_public_key: bytes (Ed25519 v1 = 32 bytes)
7 credential_epoch: u64
8 issuer_device_signing_key_id: 32 bytes
```

### 19.3 Root successor statement

Domain: `crosslab.root-successor.v1`

```text
1 schema_version: u16
2 owner_id: 32 bytes
3 current_root_key_id: 32 bytes
4 current_root_epoch: u64
5 next_root_key_id: 32 bytes
6 next_root_algorithm: u16
7 next_root_public_key: bytes
8 next_root_epoch: u64
```

Normal rotation requires valid signatures by both current and next root keys over the same transcript digest.

### 19.4 Pairing transcript

Domain: `crosslab.pairing-transcript.v1`

```text
1 pairing_profile: u16
2 protocol_major: u16
3 pairing_id: 16 bytes
4 owner_id: 32 bytes
5 inviter_device_id: 32 bytes
6 inviter_device_algorithm: u16
7 inviter_device_public_key: bytes
8 inviter_nonce: 32 bytes
9 joiner_device_id: 32 bytes
10 joiner_device_algorithm: u16
11 joiner_device_public_key: bytes
12 joiner_nonce: 32 bytes
```

### 19.5 Pairing credential acceptance

Domain: `crosslab.pairing-credential-accepted.v1`

```text
1 pairing_transcript_digest: 32 bytes
2 device_credential_signed_object_digest: 32 bytes
3 joiner_device_id: 32 bytes
4 joiner_device_key_id: 32 bytes
```

### 19.6 Trust transition

Domain: `crosslab.trust-transition.v1`

```text
1 schema_version: u16
2 owner_id: 32 bytes
3 device_id: 32 bytes
4 transition_id: 32 bytes
5 previous_revision: u64
6 new_revision: u64
7 action: u16
8 credential_epoch_context: u64
9 issuer_role: u16
10 issuer_key_id: 32 bytes
```

### 19.7 Session authentication transcript

Domain: `crosslab.session-auth.v1`

```text
1 schema_version: u16
2 owner_id: 32 bytes
3 initiator_device_id: 32 bytes
4 initiator_device_key_id: 32 bytes
5 initiator_credential_signed_object_digest: 32 bytes
6 initiator_nonce: 32 bytes
7 responder_device_id: 32 bytes
8 responder_device_key_id: 32 bytes
9 responder_credential_signed_object_digest: 32 bytes
10 responder_nonce: 32 bytes
11 negotiated_protocol_major: u16
12 negotiated_protocol_minor: u16
13 negotiated_feature_set_digest: 32 bytes
14 channel_binding_profile_digest: 32 bytes
15 channel_binding_value_digest: 32 bytes
```

The credential digest fields use `SignedObjectDigestV1` from section 17.4.

The negotiated feature set is canonicalized as ascending unique `u16` feature IDs. Its digest is:

```text
NegotiatedFeatureSetDigestV1 = BLAKE3-256(
  "crosslab.session-auth.feature-set.v1\0" ||
  feature_id_1_u16_be || ... || feature_id_n_u16_be
)
```

An empty negotiated feature set hashes only the domain label.

Channel binding profile and value are hashed separately so a profile identifier cannot be confused with binding bytes:

```text
ChannelBindingProfileDigestV1 = BLAKE3-256(
  "crosslab.session-auth.channel-binding-profile.v1\0" ||
  channel_binding_profile_bytes
)

ChannelBindingValueDigestV1 = BLAKE3-256(
  "crosslab.session-auth.channel-binding-value.v1\0" ||
  channel_binding_value_bytes
)
```

Session-authentication proofs sign the exact role label followed by the 32-byte session-auth transcript digest:

```text
initiator proof input =
  "crosslab.session-auth.initiator-proof.v1" || transcript_digest

responder proof input =
  "crosslab.session-auth.responder-proof.v1" || transcript_digest
```

After both proofs verify, the logical session identifier is:

```text
SessionId = BLAKE3-256(
  "crosslab.session-id.v1\0" ||
  transcript_digest ||
  initiator_signature_bytes ||
  responder_signature_bytes
)
```

## 20. Canonical decoding/verification rule

Security verification operates on validated domain objects, not by accepting attacker-provided "canonical bytes" as authoritative object content.

Normal verification flow:

```text
protobuf bytes
 -> bounded protobuf decode
 -> strict domain validation
 -> reconstruct canonical transcript from validated fields
 -> compute TranscriptDigestV1
 -> verify signature/MAC
 -> apply trust/policy rules
```

If wire fields cannot be represented unambiguously in the expected canonical object, verification fails.

## 21. Compatibility rules

### Compatible minor change

May include:

- optional field not used by older semantics;
- optional message type guarded by negotiated feature;
- new non-security diagnostic/error detail;
- new capability family independently negotiated.

### Requires new transcript/profile and possibly major version

Includes:

- changing meaning/type of a signed field;
- making an unsigned field security-authoritative;
- changing canonical field encoding/order/tag meaning;
- changing identity/trust semantics;
- weakening replay/channel binding;
- changing `OperationId` from correlation+binding into bearer authority;
- altering mandatory authorization behavior.

## 22. Cross-version tests

Protocol implementation must include:

- v1 peer ↔ v1 peer negotiation;
- no-common-version failure;
- highest-common-minor selection;
- unknown optional feature ignored;
- unknown required feature rejected;
- malformed/oversized frame rejection before large allocation;
- missing/unknown envelope body rejection;
- invalid identifier/enum rejection;
- control sequence duplicate/gap handling;
- duplicate request behavior by retry class;
- cancellation idempotency;
- data-stream header wrong operation/session/source rejection;
- golden canonical transcript bytes/digests/signatures for every signed v1 object.

## 23. Fuzzing obligations

Externally supplied parsers with meaningful security surface should receive fuzz targets once implemented, especially:

- frame length + protobuf decode + domain conversion;
- validated capability/operation identifiers;
- canonical transcript reconstruction from parsed domain objects;
- data-stream open headers.

Fuzzing must enforce memory/time limits suitable for CI/security jobs.

## 24. Security traceability

This specification addresses `TM-004`, `TM-005`, `TM-006`, `TM-010`, `TM-017`, `TM-018`, and `TM-022`.

Changes to public wire compatibility or canonical signing format require an ADR and explicit architecture approval.
