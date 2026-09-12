# ADR-0008: Quinn channel-binding profile v1

**Status:** Accepted  
**Date:** 2026-09-12  
**Accepted:** 2026-09-12

## Context

`docs/architecture/SESSION-TRANSPORT.md` requires every production protected transport to provide an opaque channel-binding value derived from the concrete protected connection rather than from peer-supplied application data. The binding is included in the Cross-Lab session-authentication transcript, so its derivation is security-sensitive and cross-version compatible behavior.

M8 introduces Quinn/QUIC as the first real IP transport. Quinn 0.11.11 exposes `Connection::export_keying_material`, which derives identical cryptographically strong bytes at both endpoints from the TLS session when the same label, context, and output length are used. The uploaded Quinn research tree is current `main` at package version 0.12.0 and exposes the same exporter-shaped connection API, but 0.12.0 is not the published stable dependency selected for M8.

Cross-Lab device identity remains independent of Quinn connection identity and TLS certificate identity. TLS authenticates the protected transport endpoint sufficiently to establish a confidential/integrity-protected channel; Cross-Lab owner/device credentials and trust still determine ecosystem identity and membership.

## Decision

M8 defines the following channel-binding profile:

```text
profile_id = "quic-tls-exporter-v1"
output_len = 32 bytes
label      = "EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1"
context    = "crosslab.quic.transport.v1"
```

The Quinn adapter derives exactly 32 bytes by calling `Connection::export_keying_material` after the full QUIC/TLS handshake completes.

The binding is exposed to the domain layer only as:

```text
ChannelBinding {
    profile_id: "quic-tls-exporter-v1",
    bytes: <32 exporter bytes>,
}
```

Cross-Lab session authentication continues to hash the binding profile and value into `SessionAuthTranscriptV1`; raw exporter bytes are not identity credentials and are not logged.

M8 does not use `Connecting::into_0rtt`, does not authorize application data through 0-RTT, and does not derive a Cross-Lab session from resumed early-data state. A Quinn connection is eligible for ordinary Cross-Lab session authentication only after the normal connection future completes successfully and exporter material is available.

The M8 production dependency is pinned to published stable Quinn `0.11.11`. Tokio `1.53.1` is the adapter runtime dependency. Rustls `0.23.44` and rcgen `0.14.10` are used directly only where loopback test certificate construction requires their public types; transport identity remains non-authoritative.

## Alternatives considered

### Bind to the TLS server certificate or certificate fingerprint

Rejected. A certificate is not unique to one protected connection, so it does not provide the required fresh anti-splicing channel binding. It would also risk making transport certificate identity look like Cross-Lab device identity.

### Bind to Quinn connection IDs, socket addresses, or `stable_id()`

Rejected. QUIC connection IDs and addresses are routing/lifecycle metadata, not cryptographic proof of the TLS session. Quinn's process-local stable identifier is diagnostic state, not a peer-verifiable channel binding.

### Send a random channel-binding nonce inside the Cross-Lab bootstrap protocol

Rejected. Peer-supplied application bytes are not derived from the protected handshake and therefore cannot independently bind authentication to the underlying channel.

### Derive a new binding with Cross-Lab custom cryptography

Rejected. Quinn already exposes the standard TLS exporter primitive needed for this purpose. Adding another key-derivation layer would increase security-sensitive code without improving the required property.

### Use Quinn 0.12.0 from the uploaded research tree

Rejected for M8 production. The uploaded tree is useful forward-looking research, but Cross-Lab dependency policy selects maintained published stable versions. As of 2026-09-12, docs.rs still identifies Quinn 0.11.11 as the latest published stable crate.

## Security impact

The exporter binds Cross-Lab session proofs to the exact TLS session secrets of the established QUIC connection. A proof captured on one connection cannot authenticate a fresh connection because its channel-binding transcript input changes along with the fresh nonces.

The TLS certificate used in M8 loopback tests does not grant Cross-Lab trust or capability permission. Device credentials, local trust state, protocol negotiation, role-separated proofs, policy evaluation, and operation authorization remain mandatory above the transport.

Exporter bytes are treated as sensitive session material for logging purposes: they may be hashed into the existing session transcript but are never emitted to logs or diagnostics.

0-RTT remains disabled for Cross-Lab authority, preventing replay-prone early data from bypassing fresh session authentication.

## Compatibility impact

The exact profile ID, output length, label bytes, and context bytes are compatibility-significant. Implementations that claim `quic-tls-exporter-v1` must use these exact values.

Changing any of those values requires a new channel-binding profile or a superseding ADR. A future Quinn upgrade may change internal APIs but must preserve these semantics or introduce a reviewed replacement profile.

## Operational impact

The binding requires no extra network round trip and no persistent secret beyond Quinn/TLS connection state. It is derived once after connection establishment and retained only as the opaque `ChannelBinding` value needed by the logical session.

Loopback tests generate an ephemeral self-signed certificate and explicitly trust it in the paired test client. Production certificate provisioning is not decided by this ADR and is outside M8.

## Consequences

- M8 has a concrete reviewed channel-binding mechanism instead of a placeholder transport binding.
- Quinn/TLS remains transport protection rather than the Cross-Lab identity model.
- Session replay across connections remains rejected by the existing transcript/proof design.
- The adapter can remain isolated in `transports/quic` without exposing Quinn or rustls types to identity, policy, protocol, or core session state.
- 0-RTT application/session authority remains outside Phase 1.
