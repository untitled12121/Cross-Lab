# ADR-0003: Pairing bootstrap profile v1

**Status:** Proposed  
**Date:** 2026-09-11

## Context

Cross-Lab must pair devices without assuming that LAN/BLE/USB proximity proves identity. The initial Core Simulator needs a concrete authenticated pairing profile, while future user interfaces may offer QR, BLE, USB, NFC, or numeric-code bootstraps. A short numeric code is not a high-entropy secret and must not be protected by a simple hash/MAC comparison.

## Decision

Phase 1 pairing profile v1 uses a **single-use 256-bit cryptographically random pairing secret** delivered through an out-of-band channel. Real UIs should prefer QR or another channel capable of carrying the full secret; the simulator injects the same logical secret directly.

Pairing messages exchange only public/non-secret values over the provisional transport. Both peers compute a canonical transcript containing the owner domain, pairing identifier, initiator/responder roles, both fresh nonces, both device identifiers/public keys, and pairing/profile versions.

Each side proves knowledge of the pairing secret using **HMAC-SHA-256** over a domain-separated transcript hash with a distinct initiator/responder confirmation label. The pairing secret is never itself transmitted over the provisional network channel.

After confirmation, the owner-authorizing side issues the device credential. The joining device proves possession of its private device key over the confirmed transcript and credential digest before trust is committed.

A pairing invitation is single-use. Success, explicit cancellation, timeout, or a confirmation/authentication failure consumes the invitation; retry requires a new secret.

Low-entropy numeric-code pairing is **not part of profile v1**. If added later, it requires a separately reviewed PAKE/equivalent protocol and online-guessing controls.

## Alternatives considered

### Treat transport encryption/proximity as pairing authentication

Rejected. It would make Cross-Lab trust dependent on transport identity and discovery context.

### Simple six-digit numeric code + hash/MAC

Rejected. The secret space is too small and permits offline/online guessing depending on transcript exposure and protocol behavior.

### Introduce a full additional secure-channel/AKE framework during pairing

Possible, but unnecessary for the initial owner-confirmed flow because the exchanged device credentials/public keys are not confidential. The high-entropy out-of-band secret authenticates transcript integrity; later session encryption is handled by the secure-session/transport profile.

## Security impact

The one-time high-entropy secret prevents an on-path attacker who does not observe the out-of-band channel from substituting pairing transcript values. Fresh nonces and single-use invitation state prevent replay. Direction-specific confirmation prevents reflection between roles.

Compromise of the out-of-band pairing secret before pairing completes can allow an attacker to participate in that pairing attempt; UI design must therefore protect and expire invitations.

## Compatibility impact

The pairing profile/version is explicit. Future PAKE-based numeric-code or platform-native bootstrap profiles can coexist without reinterpreting profile v1.

## Operational impact

Phase 1 requires a maintained HMAC/SHA-256 implementation and secure random generation. Exact Rust crate versions are verified at implementation time.

## Consequences

- Phase 1 has a concrete pairing bootstrap without tying trust to Quinn/BLE/LAN/USB identity.
- QR/bootstrap implementations can carry the full one-time secret later.
- Numeric pairing codes remain safely deferred.
- Pairing confidentiality is not promised by the bootstrap protocol; non-public application payloads must not be sent before a secure authorized session exists.
- This ADR requires owner approval during the Phase 0 architecture review before status changes to Accepted.
