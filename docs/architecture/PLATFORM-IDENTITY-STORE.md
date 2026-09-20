# Platform Identity Store — Linux and Android Design

**Status:** Design baseline  
**Date:** 2026-09-20  
**Architecture:** Master Architecture revision 2.6, ADR-0013, ADR-0015

## 1. Scope

This document defines the minimum platform behavior needed before normal Add Device / pairing can replace development provisioning.

It covers identity/trust persistence and signing-provider resolution only. It does not define a general settings database, recovery UI, cloud sync, transport discovery, or capability state.

## 2. Logical state

A production platform identity store must be able to reconstruct:

```text
Owner authority
  OwnerRootRecord
  highest accepted root epoch
  active Device Signing delegation + accepted epoch
  active Administrative delegation + accepted epoch
  active Recovery delegation + accepted epoch

Local device
  DeviceId
  DeviceCredential
  accepted credential epoch
  signing-provider reference

Trust graph subset persisted locally
  TrustRecord[]
  highest accepted trust revision per DeviceId

Store currentness
  schema version
  store revision
  backend currentness/rollback anchor
```

Raw private signing keys are not members of this logical public snapshot.

## 3. Required adapter operations

The concrete adapters must support the following logical operations. Exact Rust/Kotlin/native APIs may differ.

```text
load_validated_identity()
resolve_signer(role/key id)
commit_pairing(...)
commit_revocation(...)
commit_credential_rotation(...)
commit_authority_transition(...)
wipe_local_identity()
```

Every mutation is an atomic logical security transition. "Write a credential now, write trust later" is invalid if a crash could leave a durable state that changes authorization semantics.

## 4. Validation on startup

Startup validation occurs before authenticated runtime/session acceptance:

1. validate store schema;
2. validate owner root/delegation chain;
3. check accepted authority epochs/currentness;
4. validate local device credential;
5. validate every persisted trust record;
6. resolve required signing-provider references;
7. verify provider public keys match expected KeyIds;
8. verify backend currentness anchor;
9. only then expose identity/trust state to runtime.

Failure is explicit and recoverable through a future recovery/re-enrollment UX. It does not create a replacement identity silently.

## 5. Android direction

### 5.1 Key protection

Android's standard Keystore API provides non-exportable/hardware-backed protection for supported key algorithms and can protect AES keys used for GCM encryption/decryption.

The current standard Android `KeyProperties` list does not expose Ed25519 as the normal Android Keystore key algorithm, while Cross-Lab identity profile v1 is Ed25519.

Therefore the baseline Android investigation is:

```text
Android Keystore AES wrapping key
        │
        └── encrypts a Cross-Lab Ed25519 software signing seed/blob
                  │
                  └── loaded only inside the narrow Android identity adapter
                      and exposed to shared logic only as SigningProvider
```

If a device/provider offers verified direct Ed25519 secure-key support, an adapter may prefer it after compatibility/evidence work. It is not assumed by the shared architecture.

### 5.2 Storage placement

Secret-bearing wrapped key blobs and rollback/currentness material must be excluded from ordinary Auto Backup/device transfer unless a reviewed migration flow explicitly enables them.

Public identity/trust metadata may be stored in app-private storage, but restoring public metadata without the matching protected provider/currentness state must fail closed.

### 5.3 Lifecycle

Expected cases to test:

- normal process restart;
- background/foreground;
- device reboot;
- screen-lock/user-auth requirements where configured;
- application update;
- application data clear;
- uninstall/reinstall;
- backup/restore to the same device;
- device-to-device restore;
- Keystore key invalidation/deletion;
- StrongBox unavailable or falling back to TEE/software security level.

The adapter records the actual protection/security level for diagnostics without changing Cross-Lab identity semantics.

## 6. Linux direction

### 6.1 Secret protection

The freedesktop Secret Service API is a candidate desktop-session backend because it provides an interoperable GNOME/KDE secret-storage interface and may require user unlock.

It is not assumed to exist on every Linux system and does not by itself establish rollback resistance.

The Linux adapter therefore needs a backend policy such as:

```text
verified stronger hardware-backed provider
    ↓ preferred where implemented
desktop Secret Service protected wrapping secret
    ↓ supported interactive desktop baseline
no verified provider/currentness backend
    ↓ fail production identity setup; development provisioning remains separate
```

The kernel key retention service is useful for runtime/session-held secrets but is not treated as the durable production owner identity store by itself.

### 6.2 Session/headless behavior

A desktop Secret Service may be unavailable or locked before the user login session is ready. Cross-Lab must surface this as "identity store locked/unavailable" instead of generating a new identity.

Headless/server Linux requires a separately reviewed backend rather than silently storing raw keys in files.

## 7. Currentness and rollback

Neither Android app-private encrypted files nor a desktop secret service automatically prove that the newest valid trust state is loaded.

Backend implementation work must define a currentness anchor and test rollback attempts such as:

- replacing the metadata snapshot with an older valid copy;
- restoring an older app backup;
- replaying an earlier trusted state after revocation;
- replaying an older authority/delegation epoch;
- restoring an old credential epoch.

Until these tests pass for a platform backend, Cross-Lab must not claim rollback-resistant production identity persistence on that platform.

## 8. Crash consistency

For each mutation, implementation tests must inject failure between every durable step.

After restart, the result must be exactly one of:

- the prior fully valid state; or
- the new fully valid state; or
- explicit fail-closed recovery-required state.

A mixed authorization state is not allowed.

## 9. Logging and diagnostics

Allowed:

- public OwnerId/DeviceId/KeyId fingerprints;
- schema/store revisions;
- backend type and coarse protection level;
- typed error category.

Forbidden:

- raw/wrapped private key bytes;
- pairing secrets;
- recovery secrets;
- keystore authentication material;
- full secret-service payloads;
- sensitive platform handles/tokens.

## 10. Implementation sequence

1. add the implementation-neutral store contract and deterministic in-memory conformance tests;
2. implement Android protected-key/provider adapter plus backup exclusions;
3. implement Android atomic/currentness persistence and negative rollback tests;
4. implement Linux desktop provider adapter;
5. implement Linux atomic/currentness persistence and negative rollback tests;
6. only then wire normal Add Device / pairing UI to durable production identity.


## 11. External platform evidence checked

The design was checked against current platform documentation before choosing adapter directions:

- Android Keystore security model and non-exportable/hardware-backed key behavior: https://developer.android.com/privacy-and-security/keystore
- Android `KeyProperties` algorithm/security-level surface: https://developer.android.com/reference/android/security/keystore/KeyProperties
- Android AES-GCM Keystore examples: https://developer.android.com/reference/android/security/keystore/KeyGenParameterSpec
- Android Auto Backup include/exclude behavior: https://developer.android.com/identity/data/autobackup
- freedesktop Secret Service API: https://specifications.freedesktop.org/secret-service/latest/
- Linux kernel key retention service: https://docs.kernel.org/security/keys/core.html

These sources support the adapter constraints above but do not prove Cross-Lab's eventual rollback/currentness guarantees. Those guarantees require backend implementation and negative testing.
