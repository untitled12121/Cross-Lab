# ADR-0006: Introduce a focused `crosslab-crypto` crate for Phase 1

**Status:** Proposed  
**Date:** 2026-09-11

## Context

The Master Architecture initially deferred a dedicated crypto crate until multiple independent consumers required a stable cryptographic API. Phase 0 specifications now require the same domain-separated canonical transcript builder/digest primitives, signature profile wrappers, secure random generation, and key-fingerprint logic across identity credentials, pairing/trust transitions, session authentication, and later recovery commands.

Keeping these primitives inside `crosslab-identity` would make policy/protocol/session code depend on identity for generic cryptographic mechanics. Keeping them inside `crosslab-protocol` would force identity credential signing/verification to depend outward on wire-protocol code, contradicting the intended dependency direction.

## Decision

Introduce a small foundation crate in Phase 1:

```text
crates/crypto  -> package crosslab-crypto
```

It owns only implementation-neutral cryptographic building blocks required by multiple domain crates:

- cryptographically secure random byte generation interface/helpers;
- identity cryptographic profile identifiers;
- Ed25519 sign/verify wrappers for profile v1;
- BLAKE3 domain-separated digests/fingerprints;
- canonical transcript v1 builder/encoding/digest primitives;
- constant-time verification helpers where required;
- secret/private-key wrapper types needed to prevent accidental logging/exposure.

It does **not** own owner/device credentials, trust records, policy, protocol messages, sessions, sockets, persistence, platform key stores, or recovery/update business semantics.

Object-specific canonical field mappings remain with the domain that owns the signed object:

- identity crate: owner delegations, device credentials, root successor;
- policy/trust domain: trust transitions where owned there;
- protocol/session code: pairing/session authentication transcripts;
- recovery domain later: recovery commands.

The protocol crate still owns wire schemas and compatibility; it no longer needs to be the implementation owner of every canonical cryptographic primitive.

## Alternatives considered

### Keep generic crypto primitives private inside `crosslab-identity`

Would preserve the original four-crate workspace but creates an artificial dependency from unrelated protocol/session security code to identity implementation details.

### Put all signing/canonicalization in `crosslab-protocol`

Would make identity credentials depend on an outward wire layer or split credential verification awkwardly between crates, violating the intended inward dependency direction.

### Duplicate small crypto helpers per crate

Rejected because security-sensitive canonical encoding/domain separation must have one reviewed implementation and test-vector set.

## Security impact

Centralizing narrow cryptographic mechanics reduces duplicate canonical encoders, inconsistent domain separation, and accidental secret exposure. The crate remains deliberately small so it does not become a general security god-module.

## Compatibility impact

No wire compatibility change. Canonical transcript bytes remain defined by the Phase 0 specification; this ADR only places the shared implementation at the correct dependency layer.

## Operational impact

Initial dependency direction becomes:

```text
crosslab-crypto
      |
      v
crosslab-identity
      |
      v
crosslab-policy
      | \
      |  \
      v   v
crosslab-protocol
      |
      v
crosslab-core
      |
      v
crosslab-sim
```

`crosslab-protocol` may also depend directly on `crosslab-crypto` for transcript/digest helpers.

## Consequences

- The Phase 1 workspace has five foundation crates instead of four.
- Generic cryptographic primitives have one review/test boundary.
- Domain semantics remain outside the crypto crate.
- The Master Architecture crate-responsibility/dependency sections should be amended after this ADR is accepted.
- This ADR requires owner approval during the Phase 0 architecture review before status changes to Accepted.
