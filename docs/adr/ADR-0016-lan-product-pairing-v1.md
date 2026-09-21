# ADR-0016: Bounded LAN discovery and provisional QUIC pairing channel v1

**Status:** Accepted  
**Date:** 2026-09-21  
**Accepted:** 2026-09-21

## Context

Cross-Lab now has:

- the ADR-0003 authenticated pairing state machine;
- the ADR-0014 `crosslab:pair:v1:` QR/bootstrap envelope;
- production Linux and Android identity stores;
- the shared product pairing coordinator with durable reciprocal trust persistence.

The remaining gap for normal Linux ↔ Android enrollment is a bounded local route-discovery mechanism and a protected provisional channel on which the existing pairing messages can run.

Discovery and transport identity must not become Cross-Lab trust authority. A scanned QR already carries the high-entropy single-use pairing secret and the expected owner/inviter device identifiers.

## Decision

### Discovery profile

Product pairing v1 uses DNS-SD/mDNS on the local network with service type:

```text
_crosslab-pair._udp.local.
```

The service is advertised only while an inviter has a current pending pairing invitation.

The DNS-SD instance name is derived only from the non-secret random `PairingId`:

```text
p-<32 lowercase hexadecimal PairingId>
```

TXT metadata is limited to:

```text
v=1
```

The advertisement does **not** contain:

- `OwnerId`;
- `DeviceId`;
- device/user names;
- the pairing secret;
- public/private identity keys;
- trust state;
- capability data.

The UDP port from DNS-SD is routing metadata only.

The inviter stops advertising when the invitation is consumed, cancelled, expired, or the pairing listener shuts down.

The joiner starts discovery only after a valid ADR-0014 bootstrap is scanned/decoded, filters for the exact expected PairingId-derived instance, and stops after a bounded timeout or successful resolution. Cross-Lab does not perform an unbounded background LAN scan.

Active address-range scanning is not introduced.

### Android discovery behavior

Android uses the platform `NsdManager`/DNS-SD API rather than adding a second Java mDNS stack.

For Android versions/networks where multicast reception requires it, discovery may hold a `WifiManager.MulticastLock` only for the bounded discovery lifetime and releases it on success, cancellation, timeout, lifecycle stop, or error.

The current app targets API 36. When Cross-Lab raises its target/API behavior into Android 17/API 37 local-network permission enforcement, the consuming milestone must add and test the appropriate user-visible permission flow rather than silently bypassing it.

### Provisional QUIC pairing channel

The local product pairing channel uses Quinn/QUIC with application protocol:

```text
crosslab-pairing-v1
```

The channel is intentionally separate from the authenticated Cross-Lab session endpoint because durable peer trust does not exist yet.

For each invitation, the inviter creates an ephemeral self-signed TLS certificate and key used only by that invitation listener. The material is discarded when the listener closes.

The joiner verifies the TLS handshake signature and negotiated ALPN but does not treat the ephemeral certificate name/key as Cross-Lab identity authority. Certificate-chain/name trust is intentionally not the owner/device trust decision.

Cross-Lab authentication remains the ADR-0003 transcript/HMAC flow anchored by the scanned 256-bit secret, followed by owner-authorized credential issuance and device proof-of-possession.

The pairing secret is never sent over the network.

### Bounds

The v1 channel is bounded to:

- one active joiner connection per invitation;
- one bidirectional pairing stream;
- TLS 1.3;
- no 0-RTT;
- existing bootstrap frame limit (64 KiB payload);
- bounded connect/bootstrap deadlines;
- explicit cancellation/close;
- no general capability/data streams before pairing succeeds.

A second connection or out-of-sequence message is rejected/closed.

### Message flow

The provisional channel carries the existing coordinator sequence:

1. joiner hello;
2. inviter hello;
3. joiner transcript confirmation;
4. inviter transcript confirmation;
5. owner authority bundle + issued joiner credential;
6. joiner credential-acceptance proof;
7. inviter atomically persists peer trust;
8. inviter reciprocal trust bundle;
9. joiner atomically persists joined owner/local credential + inviter trust;
10. joiner persistence acknowledgement;
11. inviter final acknowledgement/close.

Local persistence is atomic under ADR-0015. Cross-device atomic commit is **not** claimed. If either device fails between local commits/acknowledgements, a one-sided durable trust record may exist, but no authenticated normal session is allowed unless both devices independently possess valid current reciprocal trust. Recovery/cleanup of incomplete enrollment remains fail-closed.

## Security rationale

An active LAN attacker may redirect discovery or terminate the provisional TLS connection because the ephemeral certificate is not a pre-established owner trust anchor.

That attacker still does not know the QR-delivered high-entropy pairing secret and therefore cannot produce valid ADR-0003 transcript confirmations or the later device proof-of-possession. Pairing fails closed.

TLS still protects the provisional exchange against passive observation and ordinary packet modification on the selected route.

Discovery data, source address, DNS-SD instance, TLS certificate, and QUIC connection identity remain routing/channel metadata only.

## Alternatives considered

### Active subnet scanning

Rejected. It is noisy, privacy-hostile, harder to bound, and conflicts with the Master Architecture discovery direction.

### Put IP/port in the QR code

Rejected for v1. It couples the security bootstrap to a transient route and makes network changes/rebinding brittle.

### Treat the ephemeral TLS certificate as device identity

Rejected. It violates the invariant that Cross-Lab identity is independent of transport.

### Use an unencrypted UDP/TCP pairing exchange

Rejected. The pairing transcript is cryptographically authenticated, but the provisional channel should still provide confidentiality/integrity against passive observers and non-authenticating middleboxes.

### Promote Iroh for local product pairing

Rejected for this milestone. ADR-0009 retains Quinn as the verified local/LAN baseline; Iroh remains the remote/NAT/relay substrate.

## Compatibility impact

This ADR adds a product discovery/channel profile but does not change the ADR-0003 canonical pairing transcript, ADR-0014 QR payload, normal session authentication, or durable identity/trust formats.

The DNS-SD service type, instance derivation, ALPN, and product-pairing wire envelope become v1 compatibility surfaces.

## Consequences

- QR scanning can lead to a real local route without embedding addresses in trust data.
- LAN discovery remains short-lived and narrowly scoped.
- Pairing traffic remains protected while Cross-Lab identity is still established by the pairing protocol itself.
- Product pairing can be tested end-to-end on Linux + Android without development provisioning.
