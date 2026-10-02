# ADR-0022: Durable owner-authorized product device revocation

**Status:** Accepted
**Accepted:** 2026-10-02 (owner approval)  
**Date:** 2026-10-02  
**Scope:** Phase 2 Linux and Android production identity stores

## Context

`ProductIdentityState` schema v2 durably stores credential plus signed initial `PairingTrustTransition` for each peer, but cannot represent the terminal `Revoked` trust state after restart. The Phase 1 signed `TrustTransition` and runtime revoke path exist; the product path has no owner-facing durable removal/revocation. Simply deleting a trusted peer or toggling a local boolean creates re-enrollment/rollback and provenance ambiguity.

## Decision

- Distinguish **disconnect** (session only), **revoke** (terminal trust for a concrete `DeviceId`), and **forget presentation/metadata** (only after revocation retention requirements are met). Never equate offline status with revoked trust.
- Introduce a versioned `ProductIdentityState` schema v3 that preserves v1/v2 load support and v2 peer pairing evidence, with **optional fully signed `TrustTransition` revocation evidence** per previously trusted peer. Retain the verified credential and initial pairing transition as immutable historical authority evidence. Bounded revocation/tombstone list shares the existing peer cap; no implicit deletion.
- Decode verifies v3 lengths, uniqueness, owner/device match, monotonic revision, exact transition action, authorized signer role/currentness, valid signature, and transition against the initial trust record. Reject malformed, contradictory or stale state. Recreate `TrustRecord` as trusted or revoked from validated signed evidence; revoked entries are never advertised as normal-session trust.
- A product owner action selects the exact **full** `DeviceId` from a validated durable peer list. UI displays a shortened ID only for identification; the backend always passes the full typed identifier and requires owner confirmation. Only available active Owner Root / delegated Device Signing or Administrative authorization may issue a valid transition; an ordinary device signer cannot revoke solely by possessing its own key. Android joiners without the relevant authority surface a disabled/unavailable control, never fabricate authority.
- Persist-before-apply: obtain current protected store snapshot, derive and sign the transition, atomically CAS-commit v3 through ADR-0015 currentness/rollback protection, **then** invalidate local session/operations/discovery candidate and update presentation. On failed/ambiguous commit or runtime invalidation, fail closed and reload durable state before admitting new protected work. Remote acknowledgement is not required to protect the local owner node.
- Existing owner policy exact rules may remain in a separate ADR-0019 store, but must never reauthorize a revoked peer. Cleanup of associated partial transfers/notification subscriptions/audit metadata respects their separate retention policies.
- Re-pairing the same revoked `DeviceId` is forbidden. Enrollment after revocation requires a new logical `DeviceId`. No recovery/private keys are exposed to UI or ordinary agents.

### Exact product identity v3 payload

Use the existing `CLPIDV1\\0` magic and fixed base v2 layout, with schema `3` in the existing big-endian `u16` field. The peer count remains a bounded big-endian `u32` and each peer retains its exact v2 credential + pairing-evidence fields. Immediately after each peer, v3 writes a **single byte** revocation tag:

- `0` = no revocation; no trailing fields for that peer.
- `1` = one terminal revocation, followed by its `TransitionId` (32 bytes), `previous_revision` (`u64_be`), `new_revision` (`u64_be`), `credential_epoch_context` (`u64_be`), `issuer_role` (`u16_be`), `issuer_key_id` (32 bytes), and Ed25519 signature (64 bytes).

The signed `OwnerId` and `DeviceId` fields are reconstructed from this same peer's verified credential and paired record. The transition schema/action are the existing immutable `TrustTransition v1` / `Revoke`. Every reconstructed transition is verified against the established `TrustRecord` and currently supported owner/delegated authority. Unknown tags, roles, invalid signatures, mismatches, malformed lengths, duplicate device IDs, unsupported currentness or trailing data fail closed. The current product authority snapshot supports Owner Root and Device Signing verification; an Administrative transition is not accepted without a matching active administrative delegation.

To minimize migrations, snapshots without revocation continue emitting schema v2. At first committed terminal revocation, the entire snapshot emits v3; the previous pairing proof remains intact. A revoked `DeviceId` is retained as a tombstone (subject to existing max peer bound) and cannot be paired again.

## Alternatives considered

- **Remove trusted peer from the v2 vector:** rejected (deletion loses signed tombstone and explicit revocation semantics).
- **Store a boolean in a preferences file:** rejected (no signer authority or rollback evidence).
- **Only runtime revoke:** rejected (restart would restore ordinary trust).
- **Force remote acknowledgement:** rejected (lost/offline devices must be locally revocable).

## Security impact

Denial must be immediate and persistent, with anti-rollback currentness, signed issuer provenance, invalidation of active operations and prevention of fresh authentication. Test signers with insufficient authority, corrupt signatures, stale writer/revisions, restart/rollback, active transfer interruption, unknown ID, double revocation and offline revocation.

## Compatibility impact

The v3 identity payload is a new **persistent security format**; precise encoding and regression vectors must be reviewed before acceptance. Existing v1/v2 records continue decoding, but new writes need atomic forward migration that retains previous evidence. No normal-session wire change is needed.

## Operational impact

Use existing `TrustTransition`, `ProductIdentityState`, Linux Secret Service identity-store CAS and Android Keystore/AtomicFile store. Keep platform UX adapters separate from the Rust security mutation service. Verify physical reconnect denial after owner revokes.

## Consequences

Owner controls can be durable without disguising deletion as revocation. Schema v3 implements this approved trust format with explicit security regression tests. Product promotion still requires verified integration and real-device reconnect-denial evidence.
