# Product Pairing and Platform Signing Foundation

**Status:** Active  
**Approved:** 2026-09-20  
**Base:** `c6ea087270d3f46206840f8e09c3fe95b30a63a0`

## Goal

Prepare Cross-Lab for normal Add Device / pairing without promoting development provisioning into production identity.

## Task 1 — Signing-provider boundary

- Accept ADR-0013.
- Add a small fallible signing-provider interface in `crosslab-crypto`.
- Make software `SigningKey` implement it.
- Refactor identity, approval, trust, pairing, and session-authentication signing paths to consume the provider.
- Preserve all existing cryptographic/wire vectors.

## Task 2 — Product pairing bootstrap envelope

- Add a versioned presentation/bootstrap payload for the accepted pairing profile.
- Keep the 256-bit secret single-use and out of normal network messages.
- Keep transport/discovery hints explicitly non-authoritative.
- Do not add a short numeric-code fallback.

## Task 3 — Platform identity-store design

- Define platform adapter responsibilities for authority metadata, trust state, and signing-provider handles.
- Android and Linux implementations require separate evidence for confidentiality, lifecycle, backup/restore behavior, and rollback handling.
- Do not claim production persistence until those adapters are implemented and verified.

## Verification

- Full Rust format/check/clippy/test gate.
- Existing pairing/session golden vectors unchanged.
- Development provisioning still builds.
- No private key bytes added to UI/FFI/protocol types.
