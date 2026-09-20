# ADR-0013: Platform signing-provider boundary

**Status:** Accepted  
**Date:** 2026-09-20  
**Accepted:** 2026-09-20

## Context

Cross-Lab identity profile v1 uses Ed25519 for owner authority, delegated authority, device credentials, pairing proof-of-possession, owner approvals, trust transitions, and session-authentication proofs.

The current Rust APIs accept concrete software `SigningKey` values. That is sufficient for simulator/development provisioning, but it makes production platform integration depend on exporting private key bytes into shared Rust code.

The architecture already requires OS/hardware-backed non-exportable storage where practical and explicitly requires domain APIs to remain capable of signing through a key-provider abstraction. Product pairing and durable production identity now need that boundary before platform-specific secure storage can be integrated safely.

This ADR does not change Ed25519, identifier formats, credential formats, pairing transcripts, trust semantics, recovery semantics, or transport identity.

## Decision

Cross-Lab introduces a small implementation-neutral signing-provider boundary in `crosslab-crypto`.

A signing provider exposes only:

- the public verifying key for the signing identity;
- message/digest signing operations;
- typed signing failure.

It does **not** expose private key bytes.

The existing in-memory/software `SigningKey` implements the provider and remains valid for deterministic tests, simulator work, and explicit development provisioning.

Identity, trust, approval, pairing, and session-authentication APIs that perform signatures consume the provider boundary instead of requiring the concrete software key type.

Provider operations are fallible. Higher-level APIs map signing failure into their existing typed domain error surfaces and fail closed.

Platform adapters may later implement the same boundary using:

- hardware-backed/non-exportable signing when the platform supports the required Cross-Lab algorithm/profile;
- platform-protected wrapped software keys when direct non-exportable Ed25519 is not reliably available;
- explicitly test-only software keys for simulator/development workflows.

The exact Linux, Android, macOS, Windows, and iOS storage/backing mechanisms are platform-adapter decisions and require their own review when they create new persistence, privilege, backup/restore, or rollback semantics.

Private key export is never added merely to make a platform provider fit this API.

## Alternatives considered

### Keep concrete `SigningKey` in domain APIs

Rejected. It makes software key material an architectural assumption and blocks clean platform-backed implementations.

### Make platform adapters return raw secret bytes to Rust

Rejected. It widens secret exposure and prevents non-exportable providers.

### Change the identity cryptographic profile now

Rejected. Ed25519 profile v1 is already accepted and interoperable. Platform storage constraints do not justify silently changing identity semantics.

### Put platform keystore logic in identity/domain crates

Rejected. Secure storage and OS APIs belong behind platform adapters. Identity semantics remain platform-neutral.

## Security impact

The boundary reduces the amount of code that can obtain private key material and makes signing failure explicit.

A provider must never log secret material, authentication prompts, platform key handles that act as credentials, or signed sensitive payloads beyond existing audit policy.

Provider-backed signing does not itself prove secure persistence or rollback resistance. Those properties must be established by the concrete platform storage design.

## Compatibility impact

No wire format, canonical transcript, signature algorithm, identifier, credential, trust record, pairing profile, or protocol version changes.

Existing software keys remain source-compatible at call sites after normal Result propagation because `SigningKey` implements the provider.

## Operational impact

Some constructors/issuers that were previously infallible become fallible where an actual signing operation occurs.

Platform providers may block on OS authentication or secure-element access. Callers must keep those operations outside latency-sensitive UI rendering paths and must not hold unrelated locks while prompting/signing.

## Consequences

- Cross-Lab signing semantics remain centralized and typed.
- Platform secure storage can be introduced without making private key bytes a shared API.
- Development provisioning remains supported without defining production storage.
- Production pairing can depend on a provider handle rather than a concrete exported software key.
- Durable authority-state persistence, backup/restore behavior, rollback resistance, and exact platform keystore backends remain explicit later decisions.
