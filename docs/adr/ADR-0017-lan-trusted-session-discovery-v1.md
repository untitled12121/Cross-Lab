# ADR-0017: Privacy-conscious LAN trusted-session discovery v1

**Status:** Accepted  
**Date:** 2026-09-22  
**Accepted:** 2026-09-22

## Context

M10 established durable reciprocal device trust and a one-time pairing-only DNS-SD/QUIC profile. Phase 2 needs already-paired Linux and Android devices to find each other again on a local network, establish a normal authenticated Cross-Lab session, reconnect after transport loss, and expose presence/status without reusing development provisioning.

Normal-session discovery must not turn DNS-SD names, addresses, TLS certificates, or network continuity into identity/trust authority. It must also avoid publishing stable owner/device identifiers to every observer on the LAN.

The existing session contract already requires fresh credentials/trust/currentness validation, nonces, channel binding, proofs, and a fresh `SessionId` on every new connection.

## Decision

### LAN discovery profile

Trusted-session LAN discovery v1 uses DNS-SD/mDNS service type:

```text
_crosslab-session._udp.local.
```

A device advertises this service only while:

- the normal application runtime is active;
- automatic local trusted-device connectivity is enabled; and
- at least one durable trusted peer exists.

Each advertisement lifetime generates a fresh random 128-bit instance nonce. The DNS-SD instance is:

```text
s-<32 lowercase hexadecimal random nonce>
```

The instance is routing/lifecycle metadata only. It is not derived from `OwnerId`, `DeviceId`, keys, credentials, or trust records.

TXT metadata is exactly:

```text
v=1
```

The advertisement contains no:

- `OwnerId`;
- `DeviceId`;
- user/device name;
- public/private identity key;
- credential or trust revision;
- capability data;
- session identifier;
- pairing secret.

### Symmetric connection role

Both active devices advertise and browse.

For two discovered v1 instances, exactly one side initiates the connection:

```text
local instance lexicographically smaller than remote instance -> initiate
local instance lexicographically larger than remote instance  -> wait/accept
```

Equal instances are ignored as self/invalid candidates.

The random instance decides only who dials. It grants no identity or authority.

### Bounds and lifecycle

- Candidate state is bounded to at most 32 active service instances.
- Discovery is event-driven through the platform DNS-SD API while the runtime is active; Cross-Lab does not poll subnets or perform active address-range scans.
- Unknown/untrusted candidates may reach the bounded authentication handshake but are rejected before a logical session becomes Active.
- Authentication setup uses existing bounded Quinn/session deadlines.
- Failed candidates use bounded reconnect backoff: 1s, 2s, 4s, 8s, then 15s maximum while the route remains advertised.
- Network loss, runtime stop, service loss, explicit disconnect, revocation, or authority-currentness loss cancels relevant pending work.
- Network return or a newly observed candidate may restart connection attempts; every new connection is a fresh session boundary.

### QUIC/TLS channel

Normal LAN sessions use Quinn with ALPN:

```text
crosslab-session-v1
```

and TLS 1.3 only, with 0-RTT disabled.

The listener uses an ephemeral self-signed TLS certificate for channel confidentiality/integrity. The client verifies the TLS handshake signature and ALPN but does not treat that certificate/name as Cross-Lab identity authority.

Cross-Lab ordinary session authentication remains authoritative:

- current owner authority validation;
- current device credential validation;
- durable non-revoked peer trust;
- fresh nonces;
- ADR-0008 TLS-exporter channel binding;
- directional device-key proof-of-possession;
- fresh `SessionId`.

A terminating MITM cannot successfully relay authentication across two independently protected connections because the proof transcript includes the per-connection exporter binding.

### Presence semantics

Discovery means only that a compatible Cross-Lab service candidate is visible on the local network.

A specific trusted `DeviceId` is reported as **Connected/Online** only after ordinary Cross-Lab session authentication reaches `Active`.

UI may show reconnecting/offline state for an already-known trusted device, but an unauthenticated DNS-SD candidate is never attributed to that device.

## Alternatives considered

### Publish DeviceId in DNS-SD

Rejected. It simplifies routing but creates a stable LAN tracking identifier and leaks trusted-device topology.

### Publish a hash of DeviceId

Rejected. A stable hash remains linkable and is unnecessary because the authentication handshake already identifies the peer securely.

### Active subnet scanning

Rejected. It is noisy, privacy-hostile, brittle, and conflicts with the Master Architecture discovery model.

### Reuse the pairing service type

Rejected. Pairing is short-lived bootstrap discovery with different lifecycle and security semantics.

### Use TLS certificate identity as Cross-Lab device identity

Rejected. Transport identity remains non-authoritative under the Master Architecture.

### Reuse development provisioning

Rejected. Phase 2 must consume durable product identity/trust and platform signing providers.

## Security impact

The profile intentionally reveals only that a Cross-Lab session-capable service is active on the LAN plus an ephemeral random instance and route.

Unknown LAN peers cannot become trusted by discovery, address continuity, TLS certificate, or reconnect continuity. Every new connection performs fresh Cross-Lab authentication and creates fresh session/operation authority.

Candidate and retry bounds reduce resource-exhaustion exposure. Revoked or unknown devices fail before `Active`.

## Compatibility impact

The following become v1 compatibility surfaces:

- service type `_crosslab-session._udp.local.`;
- instance prefix/encoding `s-<32 lowercase hex>`;
- exact TXT profile `v=1`;
- ALPN `crosslab-session-v1`;
- role-selection rule based on instance ordering.

The existing ordinary session-authentication wire/transcript is unchanged.

## Operational impact

Linux uses the existing Avahi system service. Android uses `NsdManager` and may hold a multicast lock only while the active runtime discovery lifecycle requires it.

Future Android local-network permission requirements must be implemented explicitly when applicable.

Physical LAN behavior remains a separate owner-hardware verification gate.

## Consequences

Paired devices can discover each other without stable LAN identity advertisements, establish fresh authenticated sessions automatically, and reconnect without weakening the existing session/trust model.

The design accepts that an observer can learn that some Cross-Lab service is active on the LAN. It intentionally does not reveal which owner/device is present until the authenticated peer itself proves that identity.
