# Cross-Lab Recovery and Update Security

**Status:** Phase 0 specification  
**Milestone:** P0.8 — Recovery / Update Security  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0

## Part A — Recovery Plane

## 1. Recovery purpose

Recovery exists so the owner can perform narrowly defined defensive actions against a lost, unavailable, or revoked device without granting that device ordinary Cross-Lab ecosystem access.

Recovery is not an alternate general administration channel and must not become covert surveillance.

## 2. Recovery authority

Recovery authority is an owner-delegated key role defined by `IDENTITY-AND-KEYS.md` and is cryptographically separate from:

- ordinary device keys;
- Device Signing Authority;
- Administrative Authority;
- session transport keys;
- update signing keys.

A recovery key cannot create ordinary device membership, authorize arbitrary capabilities, or directly restore `TrustState::Trusted`.

## 3. Recovery eligibility/state

Recovery state is a separate axis from ordinary trust:

```text
RecoveryState
  Normal
  RecoveryEligible
  RecoveryOnly
```

Examples:

- an ordinary trusted device may be `Normal`;
- a device marked lost may become `RecoveryOnly` while ordinary trust is revoked;
- platform policy may report a device as not recovery-capable for selected actions.

Recovery state never converts an invalid recovery signature into authority.

## 4. Recovery command profile v1

A recovery command is a separately signed security object.

Canonical domain:

```text
crosslab.recovery-command.v1
```

Canonical fields:

```text
1  schema_version: u16
2  owner_id: 32 bytes
3  target_device_id: 32 bytes
4  command_id: 32 bytes
5  recovery_authority_key_id: 32 bytes
6  recovery_authority_epoch: u64
7  sequence: u64
8  action: u16
9  parameters_digest: 32 bytes
```

The canonical encoding/digest rules are those in `docs/protocol/PROTOCOL-V1.md`.

The command is signed by the active Recovery Authority key. Recovery parameters have an action-specific canonical schema and are referenced by `parameters_digest` so a parameter substitution changes the signed command.

## 5. Replay protection

Each recovery-capable target persists, per accepted recovery authority epoch, the highest successfully accepted recovery `sequence`.

Rules:

- a command sequence must be strictly greater than the highest accepted sequence for that authority epoch;
- duplicate/lower sequence is rejected;
- `command_id` must not have been accepted previously;
- changing recovery authority epoch requires a valid owner root delegation/rotation chain;
- a larger sequence or epoch without valid authority signatures grants nothing;
- the sequence is committed before/atomically with irreversible action where practical so reboot/crash does not reopen replay.

Wall-clock time may be additional policy metadata later but is not the sole replay defense.

## 6. Recovery action registry v1

The logical action registry reserves:

```text
1 = Status
2 = Lock
3 = Sound
4 = DisplayRecoveryMessage
5 = LocateOnce
6 = RevokeOrdinaryTrust
7 = RotateOrdinaryCredentials
8 = RecoverApprovedData
9 = Wipe
```

Support is capability/platform-dependent. Unsupported actions fail explicitly.

`LocateOnce` means a bounded owner-visible recovery request where platform policy permits. Continuous/covert location tracking is not implied.

`RecoverApprovedData` requires a separately designed encrypted-data recovery capability; the action identifier alone does not grant arbitrary filesystem access.

`Wipe` is destructive and must use explicit platform/user-visible safeguards where the OS provides them. Recovery-key compromise is therefore a high-impact threat and recovery keys require strong storage.

## 7. Recovery validation flow

```text
receive bounded recovery frame
  -> validate recovery namespace/schema
  -> validate owner/target IDs
  -> validate Recovery Authority delegation/epoch
  -> reconstruct canonical command/parameter transcript
  -> verify recovery signature
  -> check sequence + command replay state
  -> verify device recovery state
  -> verify platform supports action
  -> validate action-specific parameters/policy
  -> persist replay state as required
  -> execute narrow recovery action
  -> emit redacted audit result
```

Ordinary session trust is not consulted as a substitute for recovery authority. A device may accept a valid recovery command while rejecting ordinary sessions from the same owner's revoked devices.

## 8. Ordinary revocation interaction

`RevokeOrdinaryTrust` may cause the target/owner domain to record ordinary trust revocation through the recovery-authorized path defined by P0.4.

Recovery authority cannot silently clear revocation. Returning a recovered device to ordinary trust requires a new explicit enrollment/reauthorization process.

`RotateOrdinaryCredentials` may invalidate/replace ordinary device credentials according to an owner recovery procedure, but it does not automatically mark the device `Trusted`.

## 9. Recovery privacy and audit

Recovery activity must be visible/auditable to the owner and, where practical, visible on the target device. Logs record command/result metadata without recovery private keys, reusable secrets, recovered plaintext, or unnecessary location detail.

Recovery functionality must not expose a generic remote shell, hidden microphone/camera capture, or unrestricted data extraction under the label of recovery.

## 10. Recovery failure semantics

Reject explicitly on:

```text
UnsupportedRecoverySchema
WrongOwner
WrongTarget
UnknownRecoveryAuthority
WrongAuthorityRole
StaleRecoveryAuthorityEpoch
InvalidRecoverySignature
ReplaySequence
DuplicateCommand
UnsupportedRecoveryAction
InvalidRecoveryParameters
RecoveryStateDenied
PlatformDenied
```

Ambiguous state fails closed.

## Part B — Secure Updates

## 11. Update trust goals

Update distribution and update authorization are separate.

Cross-Lab must remain safe when a mirror/CDN/owner hub/network path is malicious or compromised, provided sufficient trusted update signing authority remains uncompromised.

The update system protects against:

- arbitrary artifact substitution;
- metadata tampering;
- rollback to older vulnerable releases;
- freeze on stale metadata;
- inconsistent metadata views;
- online signing-key compromise within the limits of role separation;
- trusted-root rotation requirements.

## 12. TUF-style role model

Subject to acceptance of `ADR-0005-update-trust-model-v1.md`:

```text
Root
  -> authorizes top-level keys/thresholds

Targets
  -> authorizes release artifacts

Snapshot
  -> binds versions/hashes of target metadata

Timestamp
  -> provides current/fresh snapshot reference

Delegated Targets
  -> optional platform/component/channel scopes
```

Distribution servers may serve all metadata/artifacts but are not signing authorities.

## 13. Production root threshold

Before the first production-secure release, the Root role uses at least 2-of-3 independently protected keys.

Root keys should be offline except during planned root metadata rotation/recovery ceremonies. Their custody/backup process must avoid storing all threshold keys in one ordinary online development environment.

Pre-release development artifacts may use a simpler non-production signing setup only if clearly identified as such and never described as satisfying production update guarantees.

## 14. Targets and channels

Targets metadata binds each artifact to at least:

```text
component identity
platform/architecture
release version
artifact length
cryptographic hashes
update channel/delegation scope
compatibility metadata required by installer
```

Stable/beta/nightly or component/platform channels should use delegated target scopes when practical so compromise of one online release key does not authorize every artifact namespace.

Update channel choice is owner/user policy; switching channels does not bypass signature/version checks.

## 15. Snapshot consistency

Snapshot metadata identifies trusted versions/hashes of targets/delegated metadata. Clients reject inconsistent metadata sets rather than combining arbitrary versions from different repository states.

## 16. Timestamp/freeze protection

Timestamp metadata has bounded validity. Clients reject expired/frozen metadata for unattended privileged installation.

Clock uncertainty/error is handled as an explicit update failure requiring safe user/admin recovery rather than silently disabling expiry checks.

Exact production expiry durations are release-operations policy established before production signing. They are not protocol constants needed by the Core Simulator.

## 17. Rollback protection

Clients persist trusted metadata version counters and installed component version state needed to detect rollback.

Normal production rule:

```text
new accepted metadata/artifact version >= required monotonic version
```

A release regression is handled by publishing a new, higher signed release version containing the desired prior code/fix rather than decreasing the trusted version counter.

Rollback/freeze checks cannot be disabled automatically after network/server errors.

## 18. Root rotation and compromise recovery

Root metadata rotation follows a TUF-style transition in which the currently trusted root authorizes the next root and the new root satisfies its own threshold policy.

Online Targets/Snapshot/Timestamp compromise is recovered by rotating the affected role through root/authorized delegation metadata and publishing new metadata versions.

Compromise of the root threshold is a severe incident requiring an explicit emergency recovery/distribution procedure outside ordinary unattended updates.

## 19. Artifact verification and install boundary

An artifact is eligible for installation only after:

1. metadata chain/role/threshold validation;
2. metadata expiry/version/rollback checks;
3. target path/scope validation;
4. length/hash verification;
5. Cross-Lab compatibility checks;
6. applicable OS vendor code-signing/notarization/driver-signing checks;
7. authorization to invoke any privileged installer/helper action.

The updater verifies before requesting privileged installation. The privileged install boundary independently validates the narrowly scoped artifact/install request and does not expose generic privileged execution.

## 20. Owner hub/update mirror

An owner hub may mirror update metadata/artifacts. Mirroring does not grant signing authority. A compromised hub can deny availability or serve stale/invalid data, but clients must reject data that fails the same trusted metadata chain, freshness, version, and artifact verification used for any other source.

## 21. Protocol compatibility during updates

Update selection must consider supported Cross-Lab protocol compatibility so an automatic update does not knowingly create an unsupported device split when compatible alternatives/rollout policy are required.

This is release policy layered on protocol compatibility; it never permits installing an untrusted artifact merely to regain interoperability.

## 22. `tough` implementation status

`tough` remains a candidate Rust implementation for TUF-style metadata when the updater is built. It is not a Phase 1 dependency and must be evaluated against current upstream maintenance, supported TUF features, platform requirements, and Cross-Lab role policy at that milestone.

The architecture depends on the TUF security model, not on `tough` public types.

## 23. Update test obligations

When implemented, tests must cover at least:

- valid update metadata/artifact;
- invalid target signature/threshold;
- wrong target delegation/scope;
- artifact hash/length mismatch;
- snapshot inconsistency;
- timestamp expiry/freeze;
- metadata version rollback;
- component version rollback;
- root rotation valid/invalid threshold;
- online role-key rotation after compromise;
- malicious/stale owner mirror;
- OS signing/installer validation failure;
- privileged install request outside allowed artifact scope.

## 24. Separation of authorities

The following private authorities are intentionally independent:

```text
Owner Root / ordinary identity authorities
Recovery Authority
Update Root / release signing authorities
Transport/session keys
Platform vendor signing identities
```

An implementation may colocate tools on an owner's machine for convenience, but credentials/roles must not be treated as interchangeable.

## 25. Security traceability

Recovery requirements address `TM-019` and related privilege/privacy threats. Update requirements address `TM-020`, plus `TM-012`, `TM-016`, `TM-017`, and `TM-022` where installation/logging/resource behavior intersects security.

Material changes to recovery authority, command replay semantics, update root thresholds, rollback policy, or update trust roles require ADR review and explicit architecture approval.
