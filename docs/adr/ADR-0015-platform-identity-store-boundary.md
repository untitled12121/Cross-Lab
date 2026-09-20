# ADR-0015: Platform identity-store boundary

**Status:** Accepted  
**Date:** 2026-09-20  
**Accepted:** 2026-09-20

## Context

Cross-Lab now has:

- an accepted owner/device identity hierarchy;
- authenticated pairing and trust state;
- product bootstrap envelope v1;
- the ADR-0013 `SigningProvider` boundary.

Normal product pairing still cannot be promoted to production until owner authority state, local device credentials, trust state, and signing-provider handles survive restart without exposing raw private keys or silently accepting stale/rolled-back security state.

This persistence boundary spans shared identity/trust semantics and native platform storage. It must remain separate from UI, transport, general application preferences, and privileged capability services.

## Decision

Cross-Lab defines a **platform identity store** as a native adapter responsibility with two distinct classes of state:

1. **Durable public/security metadata**
   - `OwnerRootRecord`;
   - accepted authority delegations and their highest accepted epochs;
   - local `DeviceCredential` and accepted credential epoch;
   - trusted/revoked `TrustRecord` values and trust revisions;
   - schema/revision/currentness metadata required to reject stale state.

2. **Signing-provider references**
   - stable platform-local references that resolve to the ADR-0013 `SigningProvider` implementations required by the local device/authority role;
   - never raw private-key bytes in the general metadata snapshot.

The identity store is not the general Cross-Lab application database. It is a narrow security-state boundary.

### Load contract

Before security-sensitive runtime state becomes usable after process start, the adapter must:

1. load the durable identity snapshot;
2. validate its schema and internal consistency;
3. reconstruct and validate `OwnerAuthorityState`;
4. verify local credentials and trust records against the loaded authority state;
5. resolve required signing-provider references;
6. verify each resolved provider's public key/KeyId matches the persisted public identity metadata;
7. verify the platform currentness/rollback anchor required by that backend;
8. fail closed if any required record, provider, or currentness check is missing or inconsistent.

A partially valid store must not silently downgrade into a new owner/device identity.

### Commit contract

Security transitions that create or change durable identity state use an atomic logical commit.

At minimum, successful product pairing must not become durable `Trusted` state unless the commit includes all state required to reproduce that decision after restart:

- owner authority state required to validate the credential;
- the local/peer credential state required by the role;
- the resulting trust record;
- updated currentness metadata/anchor.

Revocation, credential-epoch advancement, and owner-authority epoch changes follow the same rule: the durable public state and backend currentness anchor must move together or recovery must fail closed.

The exact crash-safe write protocol is backend-specific, but implementations must document and test every interruption point that can expose an old/new mixed state.

### Rollback/currentness

Authenticated encryption or a valid signature is not sufficient to prevent rollback of an older valid snapshot.

Each production backend must provide a documented currentness mechanism that lets Cross-Lab detect/reject stale security state across restart. A backend that only encrypts files but cannot establish currentness is not production-complete for owner/trust persistence.

The mechanism may be platform-specific. This ADR does not pretend that one universal rollback-resistant primitive exists on Linux, Android, Windows, macOS, and iOS.

### Backup, restore, migration, and uninstall

Backup/restore must never silently clone one logical `DeviceId` and its private identity onto another physical installation.

If restored metadata cannot resolve the original protected signing provider/currentness anchor, Cross-Lab fails closed and requires an explicit recovery or re-enrollment flow.

Platform backup systems must exclude secret-bearing wrapped-key blobs/currentness material unless a separately reviewed migration/recovery design explicitly permits them.

Ordinary app uninstall/reinstall may result in a new logical device identity. Preserving a prior `DeviceId` across reinstall is a recovery/migration feature, not an implicit backup behavior.

### UI/FFI boundary

UI and ordinary mobile FFI receive only presentation-safe/public identity and trust state. They never receive:

- private keys;
- wrapped private-key blobs;
- platform keystore handles that act as credentials;
- recovery secrets;
- pairing secrets outside the explicit bootstrap presentation/scanner boundary.

### Linux adapter direction

Desktop Linux has no single universally available hardware-backed Ed25519 facility.

The adapter therefore remains backend-capable rather than assuming one mechanism. Candidate secret/protection backends include desktop Secret Service implementations and stronger hardware/TPM-backed providers where available. A concrete production backend must document headless/session behavior, unlock prompts, backup/restore behavior, and rollback/currentness guarantees before promotion.

### Android adapter direction

The standard Android Keystore API provides non-exportable/hardware-backed key protection for supported algorithms and AES-GCM wrapping, but the current standard `KeyProperties` algorithm set does not expose Ed25519 as a normal Android Keystore key algorithm.

Cross-Lab therefore must not assume that an Ed25519 private key can always be generated as a non-exportable Android Keystore signing key. The Android adapter may use a platform-protected wrapping key around a software Ed25519 key when direct verified Ed25519 provider support is unavailable. That wrapped-key design remains subject to implementation/evidence for confidentiality, backup exclusion, lifecycle, and rollback/currentness.

StrongBox is opportunistic, not mandatory; lack of StrongBox must not silently change identity semantics.

## Alternatives considered

### Store software private keys directly beside public metadata

Rejected. It widens secret exposure and defeats ADR-0013.

### Make UI/mobile FFI own platform keystore objects

Rejected. UI/FFI is not an authority boundary and must not receive signing credentials.

### Treat encrypted files as rollback-resistant

Rejected. Confidentiality/integrity do not establish freshness/currentness.

### Use one mandatory storage technology on all platforms

Rejected. Platform security/storage capabilities differ materially, and forcing one backend would either weaken stronger platforms or exclude legitimate environments.

### Let backup silently clone device identity

Rejected. A logical device identity must not be duplicated by ordinary application backup/restore.

## Security impact

The decision narrows private-key exposure, makes restart/load validation explicit, requires fail-closed handling of missing provider/currentness state, and makes rollback a first-class acceptance criterion rather than an implicit property of encryption.

## Compatibility impact

No identity, pairing, trust, session, transport, or wire format changes.

Persistent platform-state formats become versioned implementation compatibility surfaces once concrete adapters are introduced. Their exact byte format is not selected by this ADR.

## Operational impact

Platform adapters may require user/session unlock prompts or hardware/provider availability. Those operations must run outside UI rendering and unrelated locks.

Loss of protected provider state may require re-enrollment or recovery instead of silently regenerating credentials.

## Consequences

- Product pairing has a defined persistence boundary without inventing a cloud account.
- Private signing material stays behind platform/provider boundaries.
- Production persistence cannot be claimed merely because an encrypted file exists.
- Linux and Android adapters can differ internally while satisfying one security contract.
- Exact backend implementations and durable byte formats remain separately verified work.
