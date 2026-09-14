# Cross-Lab Session and Transport Contract

**Status:** Phase 0 specification  
**Milestone:** P0.7 — Session + Transport Contract  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0

## 1. Purpose

This specification defines the logical session lifecycle, fresh peer authentication, transport channel binding, reconnect/revocation behavior, and transport-neutral interface requirements used by the Core Simulator and later Quinn adapter.

Applications and capabilities interact with a Cross-Lab logical session, not directly with Quinn, Iroh, libp2p, Wi-Fi, BLE, USB, or any other link implementation.

## 2. Layer responsibilities

```text
Cross-Lab logical session
  authenticated OwnerId / DeviceId
  negotiated protocol version
  negotiated capabilities
  policy/operation state
  control request/event lifecycle
  revocation/reconnect behavior
          |
          v
Transport contract
  protected connection properties
  opaque channel binding
  ordered control byte stream
  data streams
  optional datagrams
  cancellation / close
          |
          v
Concrete transport
  in-memory test harness / Quinn / future adapters
```

Transport security can contribute to authentication but does not replace Cross-Lab credentials/trust.

## 3. Transport security classes

The transport contract reports one of the semantic classes accepted by the session layer:

```text
TransportSecurityClass
  InProcessTest
  AuthenticatedConfidentialChannel
```

`InProcessTest` is allowed only inside deterministic simulator/test code and never for production network traffic. It models transport delivery and channel-binding semantics without claiming resistance to a hostile external network.

`AuthenticatedConfidentialChannel` means the concrete transport provides confidentiality/integrity and a cryptographically meaningful channel-binding value suitable for the session authentication transcript. The transport's own peer identity remains non-authoritative until Cross-Lab authentication succeeds.

A future transport class requires an explicit security review rather than silently weakening these properties.

## 4. Opaque channel binding

Every transport connection presented to a production logical session provides:

```text
ChannelBinding
  profile_id
  bytes
```

Requirements:

- binding is derived from the concrete protected connection/handshake rather than peer-supplied application data;
- both endpoints can obtain matching binding semantics for the same protected connection;
- a distinct protected connection produces a distinct binding with overwhelming probability or a transport-defined equivalent anti-splicing guarantee;
- binding bytes are opaque to Cross-Lab domain logic;
- Cross-Lab hashes the profile ID + bytes into the session-authentication transcript;
- a transport unable to provide the required binding is ineligible for ordinary protected sessions until a reviewed adapter design supplies an equivalent property.

Exact Quinn/rustls exporter/certificate-binding mechanics are decided in M8 after verifying current APIs. P0.7 does not invent a Quinn-specific public type.

The `InProcessTest` adapter may use a deterministic per-test binding supplied by the harness; this is test metadata, not a claim of network cryptographic security.

## 5. Session roles

Each session has stable handshake roles:

- **Initiator:** opened the logical session over the transport connection.
- **Responder:** accepted it.

Role is included in the authentication transcript/signature context to prevent reflection and ambiguous ordering.

## 6. Session authentication hello

After transport establishment but before ordinary protected control messages, each peer sends bounded authentication hello data:

```text
SessionAuthHelloV1
  owner_id
  device_id
  device_credential
  protocol_ranges
  supported_features
  required_features
  nonce: 32 random bytes
```

The hello is not authorization. It provides data required to authenticate credentials and negotiate the session.

Before proceeding each side validates:

- frame and collection bounds;
- owner domain expected for the relationship;
- the locally active owner root from `OwnerAuthorityState`;
- the current active Device Signing delegation from `OwnerAuthorityState`;
- device credential chain/signature under that current Device Signing authority;
- accepted credential epoch;
- current non-revoked trust state;
- device/public-key relationship;
- protocol compatibility.

A raw historical root/delegation supplied alongside a handshake cannot establish current authority. Missing or ambiguous current owner authority fails authentication.

## 7. Negotiation

Peers select protocol major/minor and required features according to `docs/protocol/PROTOCOL-V1.md`.

Failure to find a compatible version or satisfy required features aborts authentication before ordinary session traffic.

The negotiated version/features are included in the session authentication transcript so an on-path modification cannot silently downgrade the established session.

## 8. Session authentication transcript

Canonical domain:

```text
crosslab.session-auth.v1
```

Canonical fields:

```text
1  schema_version: u16
2  owner_id: 32 bytes
3  initiator_device_id: 32 bytes
4  initiator_device_key_id: 32 bytes
5  initiator_credential_signed_object_digest: 32 bytes
6  initiator_nonce: 32 bytes
7  responder_device_id: 32 bytes
8  responder_device_key_id: 32 bytes
9  responder_credential_signed_object_digest: 32 bytes
10 responder_nonce: 32 bytes
11 negotiated_protocol_major: u16
12 negotiated_protocol_minor: u16
13 negotiated_feature_set_digest: 32 bytes
14 channel_binding_profile_digest: 32 bytes
15 channel_binding_value_digest: 32 bytes
```

The canonical encoding/digest rules come from `docs/protocol/PROTOCOL-V1.md`.

`negotiated_feature_set_digest` is computed from the canonical ascending list of negotiated numeric feature IDs. Empty set still has its specified digest.

Channel binding profile/value are hashed with separate domain labels before inclusion so arbitrary raw transport data does not create ambiguous canonical objects.

Authority-currentness metadata is local session state and does not add fields to this v1 wire/signing transcript.

## 9. Session proof of possession

Both peers sign the same session-auth transcript digest with the private device key matching their accepted device credential, but direction/role is separated in the signed proof wrapper:

```text
InitiatorProof input:
  "crosslab.session-auth.initiator-proof.v1" || transcript_digest

ResponderProof input:
  "crosslab.session-auth.responder-proof.v1" || transcript_digest
```

For identity profile v1 the proof signature is Ed25519 subject to acceptance of ADR-0002.

A peer verifies the opposite proof using the credential-bound device public key. Wrong role, key, transcript, owner, negotiated version, nonce, or channel binding fails authentication.

## 10. Session identifier

After both proofs verify:

```text
SessionId = BLAKE3-256(
  "crosslab.session-id.v1\0" ||
  transcript_digest ||
  initiator_proof_signature ||
  responder_proof_signature
)
```

`SessionId` is correlation/session identity, not a bearer credential. It is unique to the fresh handshake because both nonces and the transport binding are included.

## 11. Session state machine

Phase 1 uses:

```text
Created
  -> Authenticating
  -> Active
  -> Closing
  -> Closed

Authenticating -> Closed   on failure/cancel
Active -> Closing          on graceful close/cancel
Active -> Closed           on transport loss/fatal protocol error
Active -> Revoked          on accepted peer revocation
Revoked -> Closed
```

Authority-currentness loss uses the existing fail-closed closing path; it does not create a second parallel session state machine.

`Revoked` is session state, separate from the device's persistent `TrustState::Revoked`.

Invalid transitions return typed errors; they are not silently coerced.

## 12. Activation

A session becomes `Active` only after:

1. the active owner root and current Device Signing authority validate from local `OwnerAuthorityState`;
2. both device credentials and current trust validate;
3. protocol/features negotiate successfully;
4. channel-binding requirements validate;
5. both directional proofs verify;
6. `SessionId` is established;
7. control-channel sequencing state is initialized.

Capability advertisement/negotiation occurs only after this authentication boundary. Capability negotiation still grants no policy authority.

## 13. Session-owned state

An active session owns/references:

```text
SessionContext
  session_id
  local_device_id
  peer_device_id
  owner_id
  peer_credential_epoch
  peer_trust_revision
  authenticated_root_key_id
  authenticated_root_epoch
  authenticated_device_signing_key_id
  authenticated_device_signing_epoch
  protocol_version
  negotiated_features
  negotiated_capabilities
  transport_security_class
  channel_binding_digest
  control send/receive sequence
  cancellation token/state
```

The authority-currentness fields record the local authentication basis. They are not peer-declared claims and are not new protocol-v1 transcript fields.

Authorized operations created under P0.5 bind to this `session_id` and source/destination identities.

## 14. Transport contract

The initial conceptual contract exposes only Cross-Lab-neutral operations:

```text
TransportConnection
  security_class()
  channel_binding()
  connection_metadata()
  open_control_channel()
  open_bi_stream()
  open_uni_stream()
  supports_datagrams()
  send_datagram(...)        optional
  receive_datagram(...)     optional
  close(reason)
  cancellation()
```

The eventual Rust API may adjust ownership/async details while preserving these semantic responsibilities.

It must not expose concrete transport types into `crosslab-identity`, `crosslab-policy`, `crosslab-protocol`, or domain session state.

## 15. Control channel

Each logical session uses one ordered reliable control channel for Phase 1 envelopes. The protocol layer applies the directional `message_seq` rules in `PROTOCOL-V1.md`.

A fatal sequence/framing/authentication error closes the logical session rather than attempting to continue in ambiguous state.

Future multiple control channels require an explicit sequencing design.

## 16. Data streams

Data streams are transport streams opened underneath an active logical session.

Before payload bytes are delivered to a capability handler:

- parse bounded `DataStreamOpenV1` header;
- validate `SessionId`;
- validate source/peer binding from the parent connection/session;
- resolve `OperationId` from local authorized-operation state;
- validate capability/version/operation/direction/index/use policy;
- re-check operation state/expiry/revision/revocation;
- re-check session trust/credential/authority currentness from local state;
- reserve the permitted stream use atomically;
- only then expose payload to the handler.

A new transport connection cannot attach itself to an old logical session/data operation in Phase 1.

## 17. Transport metadata

Transport metadata may report facts useful for policy/diagnostics, such as:

```text
transport class
local/remote endpoint description
metered/local classification from trusted local adapter
connection timing/RTT when available
```

Metadata is context, not identity. Peer-provided metadata cannot satisfy locally authoritative policy fields.

## 18. Reconnect

Phase 1 reconnect is deliberately simple:

1. old transport/session becomes closed;
2. old session-scoped control sequence state ends;
3. old session-scoped `AuthorizedOperation` records are cancelled/revoked/expired and are not transferable;
4. a new transport connection is established;
5. new channel binding and nonces are produced;
6. active owner authority, credentials, and trust are revalidated from current local state;
7. a completely fresh session-authentication transcript/proofs produce a new `SessionId`;
8. capabilities are re-negotiated;
9. protected operations require fresh authorization.

No session ticket, 0-RTT authorization, operation-ID transfer, or cross-session replay is supported in Phase 1.

## 19. Transport switch/migration

Seamless cross-transport migration is outside Phase 1. A future route migration design must preserve equivalent authentication/channel-binding/operation semantics and requires architecture/security review.

Until then, switching from Wi-Fi/QUIC to USB/BLE/future link is modeled as close + fresh connection + fresh logical session.

## 20. Identity-authority replacement during active session

Under ADR-0010, ordinary session authentication depends on the locally active owner root and Device Signing delegation.

When local `OwnerAuthorityState` accepts either:

- a successor owner root, or
- a newer Device Signing delegation,

an ordinary active session authenticated under the superseded authority is no longer current.

The high-level runtime must then:

- stop accepting new ordinary control work;
- invalidate/cancel session-scoped authorized operations;
- stop accepting new data-stream opens;
- signal active work to cancel as soon as practical;
- close the logical session/transport as soon as practical;
- require fresh authentication before ordinary work resumes.

Root replacement also clears active delegated roles while retaining their epoch floors. Fresh ordinary session authentication therefore fails closed until a strictly newer Device Signing delegation under the new root is accepted.

Administrative-only or Recovery-only authority rotation does not invalidate an ordinary device session because those roles did not authenticate it.

Phase 1 deliberately does not distinguish routine root/Device Signing rotation from compromise-response rotation for session continuation.

## 21. Revocation during active session

When local trust state accepts peer revocation:

- transition session to `Revoked`;
- reject new control requests as ordinary authorized work;
- invalidate/cancel peer-bound operations;
- stop accepting data-stream opens;
- signal active work to cancel as soon as practical;
- close transport/session without requiring peer acknowledgment;
- future reconnect/authentication rejects the revoked device.

Recovery-specific communication, if supported, uses the separate P0.8 recovery authority/namespace and is not ordinary session continuation.

## 22. Cancellation and shutdown

Every session owns explicit cancellation/shutdown state.

Requirements:

- closing the session stops accepting new work first;
- active operations receive cancellation;
- bounded queues are drained or abandoned according to operation semantics;
- transport streams/tasks are closed without indefinite wait;
- repeated close/cancel is idempotent;
- task ownership is structured so application shutdown can await termination;
- no detached background task may retain private authority indefinitely after session close.

Exact timeout durations are implementation/configuration decisions.

## 23. In-memory simulator transport

The Phase 1 deterministic in-memory adapter is a test harness, not a production network transport.

It must provide:

- ordered bounded control delivery;
- bounded stream delivery;
- deterministic disconnect/failure injection;
- backpressure behavior;
- a per-connection test channel-binding value;
- `TransportSecurityClass::InProcessTest`;
- explicit cancellation/close.

Cross-Lab identity signatures, credential verification, transcript hashing, trust, policy, replay state, session state, and authority-currentness rules use the same production domain implementations as real transports.

The test adapter must not implement toy encryption and label it network security. Confidentiality/integrity against an external network is first proven by the Quinn milestone.

## 24. First real transport requirements

The Quinn milestone must supply `AuthenticatedConfidentialChannel` semantics and prove:

- encrypted local/loopback communication;
- a reviewed channel-binding mechanism;
- Cross-Lab device authentication over that binding;
- multiplexed control/data streams;
- bounded stream handling;
- disconnect/reconnect + fresh auth;
- cancellation/shutdown;
- revocation behavior;
- authority-currentness behavior;
- network fault tests.

Quinn/rustls types stay in `transports/quic`.

## 25. Required Phase 1 tests

At minimum:

- valid trusted peers establish one active session;
- invalid credential/owner/epoch/current authority fails before Active;
- revoked peer fails before Active;
- replayed auth proof with fresh nonce/binding fails;
- wrong channel binding fails;
- wrong initiator/responder proof fails;
- protocol downgrade mismatch fails;
- `SessionId` differs on reconnect;
- old sequence/request/operation state cannot be replayed into new session;
- capability exchange occurs only after authentication;
- data stream with wrong session/operation binding fails;
- active root replacement invalidates sessions authenticated under the superseded root;
- active Device Signing replacement invalidates sessions authenticated under the superseded delegation;
- root replacement blocks fresh ordinary authentication until a strictly newer Device Signing delegation is active;
- Administrative/Recovery-only rotation does not invalidate ordinary sessions;
- active revocation cancels operations and closes session;
- queue saturation/backpressure does not create unbounded memory growth;
- cancellation during authentication/stream setup terminates cleanly;
- process shutdown with active sessions terminates owned tasks cleanly.

## 26. Security traceability

This specification addresses `TM-001`, `TM-004`, `TM-005`, `TM-009`, `TM-010`, `TM-014`, `TM-017`, `TM-018`, and `TM-022`.

Changes that make transport identity authoritative, allow session/operation authority to transfer across reconnect without fresh authentication, or weaken channel-binding/authority-currentness requirements require an ADR and explicit architecture approval.
