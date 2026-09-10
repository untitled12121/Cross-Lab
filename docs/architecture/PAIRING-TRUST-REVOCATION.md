# Cross-Lab Pairing, Trust, and Revocation

**Status:** Phase 0 specification  
**Milestone:** P0.4 — Pairing + Trust / Revocation  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0  
**Identity specification:** `docs/architecture/IDENTITY-AND-KEYS.md`

## 1. Purpose

This specification defines how a new device joins an owner trust domain, how pairing intent is authenticated, when trust becomes active, how revocation is represented, and how sessions/operations react when trust is revoked.

Pairing is not discovery. Discovery may locate a peer; only an authenticated owner-authorized pairing flow may create ordinary device trust.

## 2. Phase 1 pairing profile

Subject to acceptance of `ADR-0003-pairing-bootstrap-profile-v1.md`, Phase 1 uses a single-use 256-bit random `PairingSecret` delivered out of band.

Real products may encode the invitation in a QR code or equivalent high-capacity local channel. The Core Simulator injects the same logical bootstrap material directly.

Short numeric pairing codes are not supported by profile v1. Future numeric pairing requires a separately specified PAKE/equivalent construction and guessing controls.

## 3. Pairing invitation

The owner-authorizing device creates local invitation state:

```text
PairingInvitation
  profile_version
  pairing_id: 128-bit random
  pairing_secret: 256-bit random, secret
  owner_id
  inviter_device_id
  created_at / local deadline metadata
  state: Pending | Consumed | Cancelled | Expired
```

`pairing_secret` is never sent over the provisional network transport.

The invitation is single-use. The inviter consumes it on:

- successful pairing;
- explicit cancellation;
- timeout/expiry;
- confirmation authentication failure;
- detected transcript/profile inconsistency.

A failed/consumed invitation is not retried with the same secret.

## 4. Pairing roles

The protocol distinguishes:

- **Inviter:** already belongs to the owner domain and has access to an authority path capable of issuing/obtaining an ordinary device credential.
- **Joiner:** creates or presents a new device identity and requests membership.

Role is part of every confirmation transcript so messages cannot be reflected from one direction into the other.

## 5. Pairing exchange

The logical flow is:

```text
Inviter                                  Joiner
   |                                        |
create invitation                           |
share secret out-of-band -----------------> |
   |                                        |
   | <----- PairingHello(joiner values) ----|
   | ----- PairingHello(inviter values) --->|
   |                                        |
both build identical canonical transcript
   |                                        |
   | <----- JoinerConfirmation -------------|
verify secret confirmation                  |
   | ----- InviterConfirmation ------------>|
   |                                        | verify
issue DeviceCredential                      |
   | ----- owner root/delegation chain ---->|
   | ----- DeviceCredential --------------->|
   |                                        | verify chain/credential
   | <----- CredentialAccepted + device ----|
   |        proof of possession             |
verify possession                           |
commit Trusted record                       | commit local owner/device state
consume invitation                          | finish
```

A transport disconnection may abort the attempt. Reconnect does not reuse consumed or failed invitation state unless the specification explicitly establishes that the invitation remained Pending and no authentication failure occurred. Phase 1 may choose the simpler rule of requiring a new invitation after any disconnect.

## 6. Pairing hello fields

Each peer contributes its own values to the transcript.

The minimum canonical transcript contains:

```text
PairingTranscriptV1
  domain = "crosslab.pairing.transcript.v1"
  pairing_profile = 1
  protocol_major
  pairing_id
  owner_id
  inviter_device_id
  inviter_device_key_algorithm
  inviter_device_public_key
  inviter_nonce: 256-bit random
  joiner_device_id
  joiner_device_key_algorithm
  joiner_device_public_key
  joiner_nonce: 256-bit random
```

If additional security-relevant fields are added later, compatibility rules determine whether they are included in a new transcript/profile version. Unknown fields must never be silently omitted from a signature/MAC context when doing so changes security semantics.

## 7. Pairing confirmations

Both peers calculate the canonical transcript digest defined by P0.6 and derive directional confirmation values:

```text
inviter_confirm = HMAC-SHA-256(
  pairing_secret,
  "crosslab.pairing.inviter-confirm.v1" || transcript_digest
)

joiner_confirm = HMAC-SHA-256(
  pairing_secret,
  "crosslab.pairing.joiner-confirm.v1" || transcript_digest
)
```

Comparison is constant-time through the cryptographic implementation.

A confirmation for the wrong role, transcript, owner, pairing ID, nonce, key, or profile fails.

Pairing confirmation authenticates possession of the one-time bootstrap secret. It does not itself create long-term trust.

## 8. Owner-domain material delivered to the joiner

After bootstrap confirmation, the inviter supplies the public verification material required for the joiner to validate the owner domain and issued credential, including:

- `OwnerRootRecord`;
- active Device Signing Authority delegation;
- the joiner's `DeviceCredential`.

The joiner verifies the chain according to `IDENTITY-AND-KEYS.md`. Public owner/domain material is not treated as secret.

## 9. Joiner proof of possession

Before the inviter commits trust, the joiner proves possession of the private key corresponding to its newly issued device credential.

The joiner signs a domain-separated canonical statement containing at minimum:

```text
"crosslab.pairing.credential-accepted.v1"
transcript_digest
hash(canonical DeviceCredential signed object)
joiner_device_id
joiner_device_key_id
```

The inviter verifies the signature using the credential-bound device public key.

This prevents an attacker who only caused a public key/credential to be issued from completing trust without the corresponding private device key.

## 10. Trust record

Ordinary trust is represented explicitly:

```text
TrustRecord
  owner_id
  device_id
  state: Pending | Trusted | Revoked
  accepted_credential_epoch: u64
  trust_revision: u64
  last_transition_id
```

Phase 1 uses these states only. Risk, lifecycle, connectivity, and recovery state remain separate typed axes as required by the Master Architecture.

### 10.1 Initial trust

Successful pairing creates:

```text
state = Trusted
accepted_credential_epoch = credential.credential_epoch
trust_revision = 0
```

Trust must not become `Trusted` before:

- pairing confirmation succeeds in both directions;
- owner-authorized device credential validates;
- joiner proof of possession validates;
- the local commit succeeds.

### 10.2 Pending state

`Pending` may exist only as local transactional state. A remote peer cannot use `Pending` as ordinary trusted membership.

## 11. Trust transitions

Security-sensitive trust transitions are explicit and auditable. The logical transition record contains:

```text
TrustTransition
  schema_version
  owner_id
  device_id
  transition_id: 256-bit random
  previous_revision: u64
  new_revision: u64
  action
  credential_epoch_context
  issuer_role
  issuer_key_id
```

Security transitions use a domain-separated canonical signing transcript. The exact wire representation is defined in P0.6.

`new_revision` must equal `previous_revision + 1` for a normal accepted transition. A larger number alone does not make a transition valid; issuer authority and signature must verify.

## 12. Revocation

Ordinary revocation changes:

```text
Trusted -> Revoked
```

A valid revocation may be authorized by:

- Owner Root Authority;
- active Administrative Authority;
- active Device Signing Authority for ordinary device membership it governs;
- Recovery Authority only through the separately defined P0.8 recovery-revocation command path.

The signer role is included in the signed transition and validated explicitly.

A device cannot revoke or un-revoke itself merely by signing with its ordinary device key.

## 13. Revocation effects

When a local node accepts a valid revocation for `DeviceId`:

1. ordinary trust state becomes `Revoked`;
2. new authentication/session establishment for that device is rejected;
3. all local authorized operations owned by/bound to that device are invalidated;
4. active ordinary sessions with that device are terminated as soon as practical;
5. new control requests/data streams are rejected;
6. audit state records the transition without sensitive payloads;
7. recovery-specific behavior remains governed separately by recovery authority/state.

Revocation does not wait for the remote device to acknowledge it.

## 14. Reconnect after revocation

A revoked device presenting an otherwise valid old/new ordinary credential remains revoked. A higher `credential_epoch` cannot override `TrustState::Revoked` by itself.

Phase 1 treats ordinary revocation as terminal for that `DeviceId`. Re-enrollment after revocation requires creation of a new logical `DeviceId`.

A future explicit reauthorization/recovery design may preserve a prior `DeviceId`, but it must be a separately authorized transition and cannot silently clear revocation.

## 15. Credential rotation while trusted

When a trusted device receives a valid higher credential epoch through an authorized rotation process:

- the same `DeviceId` is preserved;
- the accepted credential epoch advances;
- prior lower credential epochs are rejected;
- trust remains `Trusted` unless a separate trust transition changes it.

Pairing and key rotation are distinct operations even if a UI later combines them.

## 16. Owner mismatch and cross-domain attempts

Pairing fails if:

- the transcript `owner_id` differs between peers;
- the inviter cannot provide a valid owner-root/delegation chain for that owner;
- the issued credential names another owner;
- the joiner attempts to substitute a different device identity/key after transcript confirmation.

Cross-Lab never silently merges owner trust domains.

## 17. Cancellation and partial failure

Pairing is fail-closed.

Before final trust commit, failure/cancellation leaves the joining peer without ordinary trusted membership. Any credential material issued during a failed attempt is not sufficient to bypass the inviter's local trust state.

The inviter consumes the invitation on security-significant failure. A new attempt uses a new invitation/secret/nonces.

Implementations must make local writes transactional enough that crash/restart cannot produce `Trusted` without the required confirmations/proof-of-possession having completed.

## 18. Audit events

Relevant events include:

```text
PairingInvitationCreated
PairingAttemptStarted
PairingConfirmed
PairingFailed
PairingCancelled
DeviceTrustEstablished
CredentialEpochAdvanced
DeviceRevoked
SessionTerminatedByRevocation
```

Audit data may contain stable IDs, public fingerprints, transition IDs, role, result, and timestamps. It must not contain pairing secrets, private keys, reusable authentication secrets, or sensitive capability payloads.

## 19. Phase 1 negative scenarios

Phase 1 must reject/test at minimum:

- wrong pairing secret;
- replayed old confirmation;
- wrong pairing ID;
- nonce substitution;
- owner mismatch;
- joiner key substitution after confirmation;
- invalid Device Signing Authority delegation;
- invalid device credential signature;
- credential-accepted signature from the wrong key;
- cancelled/consumed invitation reuse;
- trusted peer requesting unauthorized capability;
- stale credential epoch;
- forged/higher trust revision without valid authority;
- active revocation terminating operations/session;
- reconnect after revocation.

## 20. Security traceability

This specification addresses `TM-002`, `TM-003`, `TM-004`, `TM-009`, `TM-014`, `TM-018`, `TM-019`, and `TM-022`.

Changes to pairing authentication, trust-transition authority, revocation semantics, or reauthorization after revocation are architecture-sensitive and require an ADR when material.
