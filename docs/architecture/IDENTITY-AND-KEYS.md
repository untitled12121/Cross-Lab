# Cross-Lab Identity and Key Hierarchy

**Status:** Phase 0 specification  
**Milestone:** P0.3 — Identity / Key Hierarchy  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0  
**Threat model:** `security/THREAT-MODEL.md`

## 1. Purpose

This specification defines the stable owner/device identity model, authority-key roles, device credentials, credential epochs, key identifiers, rotation semantics, and private-key handling constraints required by the Core Simulator and later platform implementations.

Identity is a Cross-Lab domain concept. It is independent from IP/MAC/BLE/USB addresses, QUIC/TLS certificates, Iroh/libp2p identifiers, OS accounts, relay identities, and database row identifiers.

## 2. Identity objects

### 2.1 `OwnerId`

`OwnerId` is a stable 256-bit random identifier generated once for an owner trust domain using a cryptographically secure random source.

Properties:

- stable across ordinary owner authority-key rotation;
- not derived from an IP address, device address, username, public key, relay identifier, or transport library identifier;
- never sufficient by itself to prove ownership;
- represented canonically as 32 raw bytes in domain logic; textual display encoding is presentation-only.

### 2.2 `DeviceId`

`DeviceId` is a stable 256-bit random identifier generated once when a logical device identity is created.

Properties:

- stable across ordinary device-key rotation and credential renewal;
- unique within and outside an owner domain with cryptographic-random collision resistance;
- not authority by itself;
- not reused for a replacement/reinstalled device unless an explicit owner-authorized identity recovery/migration process restores that logical device identity.

### 2.3 `KeyId`

`KeyId` identifies a concrete public key and algorithm profile. For identity profile v1 it is:

```text
BLAKE3-256(
  "crosslab.key-id.v1" ||
  algorithm_id ||
  canonical_public_key_bytes
)
```

The canonical transcript encoding rules in the protocol/signing specification define exact field framing. `KeyId` is a fingerprint/lookup key; possession or knowledge of a `KeyId` grants no authority.

## 3. Authority hierarchy

Cross-Lab separates authority by purpose:

```text
Owner Root Identity
        |
        +-- Device Signing Authority
        +-- Administrative Authority
        +-- Recovery Authority

Device Signing Authority
        |
        +-- Device Credential A
        +-- Device Credential B
        +-- Device Credential C
```

A single private key must not be reused for all roles.

### 3.1 Owner Root Authority

The root authority anchors the owner trust domain and signs owner-authority delegations and normal root-successor transitions. It should be protected more strongly than ordinary online device keys and may be offline/hardware-backed in later platform implementations.

The root key is not used for routine peer sessions, data encryption, capability requests, or ordinary network authentication.

### 3.2 Device Signing Authority

The device-signing authority issues and rotates ordinary device credentials for the owner domain. Its key is delegated by the owner root.

It cannot issue recovery authority unless a future explicitly authorized root-level transition grants that role. It does not automatically possess administrative or recovery rights.

### 3.3 Administrative Authority

Administrative authority is reserved for owner-approved high-impact administration workflows. Holding this role does not make a process/device a recovery authority and does not bypass operation policy.

Phase 1 does not require a general administrative protocol, but the role exists in the credential model so ordinary device-signing authority is not overloaded later.

### 3.4 Recovery Authority

Recovery authority is cryptographically separate from normal trust and device-signing authority. It is used only for the recovery namespace specified in P0.8.

A recovery key cannot sign ordinary device membership merely because it is a recovery key. Any transition from recovery state back to ordinary trust requires an explicit owner-authorized re-enrollment/credential process.

### 3.5 Device key

Each device owns a device signing/authentication key used to prove possession of the device credential in Cross-Lab authentication/session protocols. The device key is not the owner root, device-signing authority, or recovery authority.

A device may later own additional keys for platform-native authentication or encryption profiles. Those keys require explicit roles and do not silently inherit the device signing key's semantics.

## 4. Cryptographic profile v1

Subject to acceptance of `ADR-0002-identity-cryptographic-profile-v1.md`, identity profile v1 uses:

```text
Signature algorithm: Ed25519
OwnerId:            random 32 bytes
DeviceId:           random 32 bytes
KeyId/fingerprint:  BLAKE3-256 domain-separated digest
```

Algorithm/profile identifiers are explicit so future platform-native/hardware-backed profiles can coexist without reinterpreting v1 credentials.

This specification does not define the session key-agreement/KDF/AEAD profile; that belongs to P0.7.

## 5. Owner root record

The locally trusted owner-root record contains the minimum information needed to identify the trust domain and current root verification state:

```text
OwnerRootRecord
  schema_version
  owner_id: OwnerId
  root_key_id: KeyId
  root_algorithm
  root_public_key
  root_epoch: u64
```

The first owner-root record is a local trust anchor created during owner-domain initialization. `OwnerId` alone never authenticates an alternative root key.

## 6. Owner authority delegation

An authority delegation binds one delegated key to one owner role.

Logical signed fields:

```text
AuthorityDelegation
  schema_version
  owner_id
  role
  delegated_key_id
  delegated_algorithm
  delegated_public_key
  delegation_epoch: u64
  issuer_root_key_id
```

The signed object uses the canonical Cross-Lab signing transcript and domain label for `authority-delegation`.

Rules:

- role is explicit and cannot be inferred from storage location or application usage;
- delegation epoch is monotonically increasing for replacement of the same logical role slot;
- a delegation signed by an unknown/wrong owner root is invalid;
- a delegation for one role cannot be replayed as another role because role and domain label are signed;
- unknown mandatory fields/profile versions fail according to protocol compatibility rules.

Time-based expiry may be added by a later schema/profile. Phase 1 security does not rely solely on wall-clock validity.

## 7. Device credential

A device credential binds a stable `DeviceId` and device public key to an owner domain.

Logical signed fields:

```text
DeviceCredential
  schema_version
  owner_id: OwnerId
  device_id: DeviceId
  device_key_id: KeyId
  device_algorithm
  device_public_key
  credential_epoch: u64
  issuer_device_signing_key_id: KeyId
```

The Device Signing Authority signs the canonical `device-credential` transcript.

The credential intentionally does **not** contain mutable capability lists, network addresses, transport identifiers, display names, battery state, current platform permissions, current integration level, presence, or route information. Those values change independently and belong to authenticated capability/session/device metadata, not stable identity authority.

## 8. Credential epochs

Credential epochs prevent silent reuse of superseded credentials.

For each `DeviceId`:

- initial credential epoch is `0`;
- ordinary rotation increments the epoch by exactly one in normal flows;
- trust state records the accepted/current epoch or minimum acceptable epoch;
- a credential below the accepted epoch is stale and rejected;
- accepting a higher epoch requires valid owner-domain authorization, not merely a larger number;
- an attacker cannot revive a revoked device by presenting a newly invented epoch without a valid issuer signature and trust transition.

Authority delegations use their own role-specific delegation epochs and do not share the device credential counter.

## 9. Device-key rotation

Normal device-key rotation preserves `OwnerId` and `DeviceId`:

```text
existing DeviceId
    |
    +-- credential epoch N   -> old KeyId
    |
    +-- credential epoch N+1 -> new KeyId
```

A new credential for the same device must be issued by a currently authorized Device Signing Authority. Once the trust state accepts epoch `N+1`, ordinary authentication using epoch `N` is rejected.

The old private key does not authorize the new key unless a pairing/rotation protocol explicitly requires proof of continuity in addition to owner authorization.

## 10. Authority-key rotation

### 10.1 Delegated role keys

Device-signing, administrative, and recovery role keys rotate by issuing a new role delegation with a higher role-specific epoch from the active owner root authority.

Consumers must reject older role epochs after the newer delegation has been accepted into authoritative local state.

### 10.2 Owner root key

Normal root-key rotation requires continuity evidence binding old and new roots, including signatures by both the current root and the proposed new root over a canonical root-successor statement.

Emergency root recovery after suspected root compromise is not equivalent to normal rotation and is specified as a recovery ceremony in P0.8. Phase 1 does not implement emergency root recovery.

Stable `OwnerId` remains unchanged across an approved root-successor transition.

## 11. Key storage and process isolation

Private keys remain local to the authority/device that owns them.

Requirements:

- never log or include private keys in audit events;
- zeroize sensitive transient buffers where supported and useful;
- avoid unnecessary serialization/copies of private key material;
- do not expose private-key types through UI, protocol, transport, or plugin APIs;
- use OS/hardware-backed non-exportable storage where practical on production platforms;
- keep domain APIs capable of signing through a key-provider abstraction later without forcing keys to be exportable;
- test keys are synthetic and never reused as production credentials.

Phase 1 may use in-memory software keys for deterministic/local simulator tests, but those keys are test-only and do not weaken production storage requirements.

## 12. Identity validation

A device identity is accepted only when all required conditions hold:

1. credential schema/profile is supported;
2. `OwnerId` matches the expected trust domain;
3. issuing Device Signing Authority is valid for that owner/role/epoch;
4. device credential signature verifies over the canonical transcript;
5. `DeviceId`, `KeyId`, algorithm, and public-key encoding are structurally valid;
6. credential epoch is acceptable for the current trust state;
7. device/revocation state permits ordinary authentication;
8. the session proves possession of the corresponding device private key.

Credential verification and proof-of-possession are separate checks.

## 13. Failure semantics

The identity layer returns explicit typed failures for at least:

```text
UnsupportedSchema
UnsupportedAlgorithm
MalformedIdentifier
MalformedPublicKey
UnknownOwner
UnknownIssuer
WrongIssuerRole
InvalidSignature
WrongOwner
StaleCredentialEpoch
UnexpectedCredentialEpoch
RevokedDevice
InvalidAuthorityEpoch
InvalidRootSuccessor
```

Callers must not convert unknown/ambiguous identity failures into an authenticated principal.

## 14. Audit semantics

Identity/security transitions may emit structured audit events containing stable identifiers, public key fingerprints, role, epoch, decision/result, and time where appropriate.

Audit events never contain private keys, raw bootstrap secrets, reusable authentication secrets, or sensitive payloads.

## 15. Test-vector obligations

Before/while P1 identity implementation lands, the repository must add deterministic vectors for:

- `OwnerId`/`DeviceId` binary/text round trips when textual encoding is defined;
- `KeyId` derivation;
- authority-delegation signature verification;
- device-credential signature verification;
- wrong-domain/wrong-role signature rejection;
- malformed key/identifier rejection;
- credential epoch progression and stale rejection;
- device-key rotation preserving `DeviceId`;
- normal root-successor verification;
- recovery key rejected for ordinary device issuance.

The vectors use fixed synthetic keys and contain no production secrets.

## 16. Phase 1 minimum

The Core Simulator requires only:

- owner-domain initialization;
- owner root + delegated Device Signing Authority;
- device identity/key creation;
- device credential issuance/verification;
- stable IDs and key fingerprints;
- credential epochs;
- proof-of-possession support for P0.7 session authentication;
- hooks for trust/revocation checks.

Administrative operations, emergency root recovery, hardware-backed stores, and platform key-provider adapters remain outside the simulator.

## 17. Security traceability

This specification directly addresses `TM-001`, `TM-002`, `TM-004`, `TM-009`, `TM-014`, `TM-019`, and `TM-022` in `security/THREAT-MODEL.md`.

Any change that makes stable identity derive from a transport/library identifier, merges recovery authority into ordinary device trust, or changes credential/key semantics requires an ADR and explicit architecture approval.
