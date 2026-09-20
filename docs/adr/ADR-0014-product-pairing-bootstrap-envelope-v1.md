# ADR-0014: Product pairing bootstrap envelope v1

**Status:** Accepted  
**Date:** 2026-09-20  
**Accepted:** 2026-09-20

## Context

ADR-0003 defines Cross-Lab pairing profile v1 as a single-use 256-bit random secret delivered out of band. The product now needs one deterministic cross-platform representation that Linux, Android, and later platforms can render as QR data or accept from another local high-capacity channel.

The representation must not turn discovery metadata, IP addresses, transport certificates, or device names into trust anchors. It must also remain simple enough to parse without adding another serialization dependency to the core pairing boundary.

## Decision

Cross-Lab product pairing bootstrap v1 uses the ASCII form:

```text
crosslab:pair:v1:<lowercase-hex-payload>
```

The fixed binary payload is:

```text
u16 pairing_profile = 1, big-endian
PairingId             16 bytes
PairingSecret         32 bytes
OwnerId               32 bytes
Inviter DeviceId      32 bytes
```

The payload is hex encoded by the canonical encoder. Decoders may accept upper- or lowercase hex but must reject malformed length, invalid encoding, and unsupported embedded profile versions.

The bootstrap code is secret-bearing presentation data. Debug output must redact it. Temporary encoding/decoding buffers are cleared where practical.

The inviter's creation/deadline state remains local and is not encoded. Expired, cancelled, consumed, or otherwise non-pending invitations must not mint a fresh product bootstrap code.

Transport/discovery hints are intentionally excluded from v1. Address discovery may occur separately, but an IP address, BLE identifier, hostname, transport certificate, or similar route hint never authenticates the owner/device and is never incorporated as authority by this envelope.

The QR renderer/scanner is a platform UI concern. Shared Rust defines and validates the envelope only; it does not own camera APIs or visual QR rendering.

## Alternatives considered

### JSON bootstrap document

Rejected for v1. It is larger, admits unnecessary stringly fields, and creates more canonicalization surface for a fixed-size security bootstrap.

### Base64url payload

Viable and more compact, but would introduce additional encoding code/dependency for a payload that is already comfortably small for QR byte mode. A future envelope version may choose another encoding without reinterpreting v1.

### Include LAN address or discovery metadata

Rejected. Route hints are mutable and unauthenticated before the pairing transcript is confirmed. Including them in the security bootstrap encourages transport identity to leak into trust semantics.

### Short numeric pairing code

Rejected by ADR-0003. Profile v1 requires the full high-entropy secret; a future numeric flow requires a separately reviewed PAKE/equivalent profile.

## Security impact

Possession of the bootstrap code reveals the one-time pairing secret and therefore must be treated like a temporary credential until the invitation is consumed/expired.

The envelope does not itself establish trust. ADR-0003 transcript confirmation, owner-authorized credential issuance, device proof-of-possession, and trust commit remain mandatory.

## Compatibility impact

The `crosslab:pair:v1:` form and fixed field order are cross-platform compatibility surface. Future incompatible changes use a new bootstrap version rather than reinterpreting v1.

No existing Quinn/session/control wire format changes.

## Operational impact

Product UIs can render the returned bootstrap string as QR data and scanners can pass scanned text into the shared decoder.

No new runtime dependency or network service is introduced.

## Consequences

- Linux/Android can share one deterministic QR/bootstrap payload.
- Pairing remains independent of discovery and transport identity.
- Secret-bearing presentation data has an explicit redaction boundary.
- The product can add QR UI without inventing a second pairing protocol.
