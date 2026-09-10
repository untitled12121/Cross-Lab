# ADR-0002: Identity cryptographic profile v1

**Status:** Proposed  
**Date:** 2026-09-11

## Context

Phase 1 requires production-grade signature primitives for owner authority, device credentials, authentication proofs, and deterministic test vectors. Cross-Lab must not implement custom cryptographic primitives and must keep domain identity independent from transport/TLS identities. The Master Architecture also requires algorithm/version fields so future migrations do not redefine `OwnerId` or `DeviceId`.

## Decision

For Cross-Lab identity profile v1:

- owner-root, delegated owner-authority, and device signing keys use **Ed25519** signatures;
- secure key generation uses the operating system cryptographic random source through a maintained Rust cryptographic library/RNG interface;
- stable `OwnerId` and `DeviceId` values are independent random 256-bit identifiers generated once per logical identity/device and are not derived from transport addresses or rotatable public keys;
- `KeyId` is a 256-bit BLAKE3 domain-separated digest of the algorithm identifier and canonical public-key bytes;
- security transcripts include explicit Cross-Lab domain/version labels before signing;
- credential structures carry an algorithm/profile identifier and schema version to permit future migration;
- key agreement, session KDF, AEAD, and transport TLS choices are not decided by this ADR and are specified in P0.7/Quinn milestones.

The logical domain model must not expose a specific Rust cryptography crate in public Cross-Lab types.

## Alternatives considered

### Derive `OwnerId`/`DeviceId` directly from public keys

This is self-authenticating but couples stable identity to key rotation. Cross-Lab requires stable device/owner identifiers across ordinary credential rotation, so identifiers remain separate from keys.

### P-256 signatures

P-256 has broad platform/hardware support and may be useful for platform-backed credentials later. Making it the only Phase 1 identity primitive would add encoding/implementation complexity that is unnecessary for the simulator. Future profiles may add platform-native algorithms without changing stable IDs.

### RSA

Provides no benefit for the initial Cross-Lab design and increases key/signature size and implementation surface.

### Custom signature or hash construction

Rejected. Cross-Lab uses maintained, reviewed cryptographic implementations and only defines domain separation/credential semantics around them.

## Security impact

Ed25519 provides deterministic signatures with a compact mature ecosystem. Stable random IDs prevent key rotation from changing domain identity. `KeyId` is only an identifier/fingerprint and never grants authority by itself. Domain separation prevents signatures intended for one Cross-Lab object/operation from being reused as another.

## Compatibility impact

All v1 credentials identify their profile/version explicitly. Future signature algorithms require a new profile and compatibility rules rather than reinterpretation of v1 bytes.

## Operational impact

Phase 1 will need a maintained Ed25519 implementation, BLAKE3, and an OS-backed secure random source. Exact crate versions are verified immediately before workspace implementation rather than fixed in this ADR.

## Consequences

- `OwnerId` and `DeviceId` survive ordinary key rotation.
- Trust always requires a credential/key relationship in addition to an identifier.
- Public domain types remain crypto-library-neutral.
- Hardware-backed/non-exportable platform keys may require additional algorithm profiles later.
- This ADR requires owner approval during the Phase 0 architecture review before status changes to Accepted.
