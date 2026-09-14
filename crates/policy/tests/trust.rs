use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError,
    OwnerAuthorityState, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    CredentialRotationError, PairingTrustTransition, PairingTrustTransitionError, TransitionId,
    TrustRecord, TrustState, TrustTransition, TrustTransitionError,
};

fn establish_initial_trust(
    credential: &DeviceCredential,
    authority: &OwnerAuthorityState,
    issuer_key: &SigningKey,
    transition_id: TransitionId,
    pairing_evidence_digest: [u8; 32],
) -> TrustRecord {
    let transition = PairingTrustTransition::issue_current(
        credential,
        transition_id,
        pairing_evidence_digest,
        authority,
        issuer_key,
    )
    .unwrap();
    transition.establish_current(credential, authority).unwrap()
}

fn trusted_record() -> TrustRecord {
    let owner_id = OwnerId::from_bytes([1; 32]);
    let root_key = SigningKey::from_secret_bytes([4; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer_key = SigningKey::from_secret_bytes([7; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        0,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();
    let credential = DeviceCredential::issue_current(
        owner_id,
        DeviceId::from_bytes([2; 32]),
        &SigningKey::from_secret_bytes([8; 32]),
        0,
        &authority,
        &issuer_key,
    )
    .unwrap();

    establish_initial_trust(
        &credential,
        &authority,
        &issuer_key,
        TransitionId::from_bytes([3; 32]),
        [9; 32],
    )
}

struct RotationFixture {
    owner_id: OwnerId,
    device_id: DeviceId,
    root: OwnerRootRecord,
    issuer_key: SigningKey,
    delegation: AuthorityDelegation,
    authority: OwnerAuthorityState,
}

impl RotationFixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x20; 32]);
        let root_key = SigningKey::from_secret_bytes([0x21; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x22; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        Self {
            owner_id,
            device_id: DeviceId::from_bytes([0x23; 32]),
            root,
            issuer_key,
            delegation,
            authority,
        }
    }

    fn trust(&self) -> TrustRecord {
        let credential = self.credential(self.device_id, 0);
        establish_initial_trust(
            &credential,
            &self.authority,
            &self.issuer_key,
            TransitionId::from_bytes([0x24; 32]),
            [0x34; 32],
        )
    }

    fn credential(&self, device_id: DeviceId, epoch: u64) -> DeviceCredential {
        DeviceCredential::issue_current(
            self.owner_id,
            device_id,
            &SigningKey::from_secret_bytes([0x25; 32]),
            epoch,
            &self.authority,
            &self.issuer_key,
        )
        .unwrap()
    }
}

#[test]
fn pairing_trust_transition_establishes_initial_membership() {
    let fixture = RotationFixture::new();
    let credential = fixture.credential(fixture.device_id, 0);
    let transition_id = TransitionId::from_bytes([0x2d; 32]);
    let pairing_evidence_digest = [0x2e; 32];
    let transition = PairingTrustTransition::issue_current(
        &credential,
        transition_id,
        pairing_evidence_digest,
        &fixture.authority,
        &fixture.issuer_key,
    )
    .unwrap();

    let record = transition
        .establish_current(&credential, &fixture.authority)
        .unwrap();

    assert_eq!(record.owner_id(), fixture.owner_id);
    assert_eq!(record.device_id(), fixture.device_id);
    assert_eq!(record.state(), TrustState::Trusted);
    assert_eq!(record.accepted_credential_epoch(), 0);
    assert_eq!(record.trust_revision(), 0);
    assert_eq!(record.last_transition_id(), transition_id);
    assert_eq!(
        transition.pairing_evidence_digest(),
        pairing_evidence_digest
    );
}

#[test]
fn pairing_trust_transition_is_bound_to_the_exact_credential() {
    let fixture = RotationFixture::new();
    let credential = fixture.credential(fixture.device_id, 0);
    let transition = PairingTrustTransition::issue_current(
        &credential,
        TransitionId::from_bytes([0x2f; 32]),
        [0x30; 32],
        &fixture.authority,
        &fixture.issuer_key,
    )
    .unwrap();
    let substituted = DeviceCredential::issue_current(
        fixture.owner_id,
        fixture.device_id,
        &SigningKey::from_secret_bytes([0x31; 32]),
        0,
        &fixture.authority,
        &fixture.issuer_key,
    )
    .unwrap();

    assert_eq!(
        transition.establish_current(&substituted, &fixture.authority),
        Err(PairingTrustTransitionError::CredentialMismatch)
    );
}

#[test]
fn pairing_trust_transition_rejects_non_initial_credential_epoch() {
    let fixture = RotationFixture::new();
    let credential = fixture.credential(fixture.device_id, 1);

    assert_eq!(
        PairingTrustTransition::issue_current(
            &credential,
            TransitionId::from_bytes([0x32; 32]),
            [0x33; 32],
            &fixture.authority,
            &fixture.issuer_key,
        ),
        Err(PairingTrustTransitionError::NonInitialCredentialEpoch)
    );
}

#[test]
fn trusted_record_starts_at_revision_zero() {
    let record = trusted_record();

    assert_eq!(record.state(), TrustState::Trusted);
    assert_eq!(record.accepted_credential_epoch(), 0);
    assert_eq!(record.trust_revision(), 0);
}

#[test]
fn verified_successor_credential_advances_epoch_revision_and_transition() {
    let fixture = RotationFixture::new();
    let mut record = fixture.trust();
    let successor = fixture.credential(fixture.device_id, 1);
    let transition_id = TransitionId::from_bytes([0x26; 32]);

    record
        .accept_successor_credential_current(&successor, &fixture.authority, transition_id)
        .unwrap();

    assert_eq!(record.state(), TrustState::Trusted);
    assert_eq!(record.accepted_credential_epoch(), 1);
    assert_eq!(record.trust_revision(), 1);
    assert_eq!(record.last_transition_id(), transition_id);
}

#[test]
fn successor_credential_rejects_stale_skipped_wrong_device_and_wrong_authority() {
    let fixture = RotationFixture::new();

    let mut stale_record = fixture.trust();
    let stale = fixture.credential(fixture.device_id, 0);
    assert_eq!(
        stale_record.accept_successor_credential_current(
            &stale,
            &fixture.authority,
            TransitionId::from_bytes([0x27; 32]),
        ),
        Err(CredentialRotationError::StaleCredentialEpoch)
    );

    let mut skipped_record = fixture.trust();
    let skipped = fixture.credential(fixture.device_id, 2);
    assert_eq!(
        skipped_record.accept_successor_credential_current(
            &skipped,
            &fixture.authority,
            TransitionId::from_bytes([0x28; 32]),
        ),
        Err(CredentialRotationError::UnexpectedCredentialEpoch)
    );

    let mut wrong_device_record = fixture.trust();
    let wrong_device = fixture.credential(DeviceId::from_bytes([0x29; 32]), 1);
    assert_eq!(
        wrong_device_record.accept_successor_credential_current(
            &wrong_device,
            &fixture.authority,
            TransitionId::from_bytes([0x2a; 32]),
        ),
        Err(CredentialRotationError::WrongDevice)
    );

    let mut wrong_authority_record = fixture.trust();
    let successor = fixture.credential(fixture.device_id, 1);
    let other_root_key = SigningKey::from_secret_bytes([0x2b; 32]);
    let other_root = OwnerRootRecord::new(fixture.owner_id, &other_root_key, 0);
    assert_eq!(
        wrong_authority_record.accept_successor_credential(
            &successor,
            &other_root,
            &fixture.delegation,
            fixture.delegation.delegation_epoch(),
            TransitionId::from_bytes([0x2c; 32]),
        ),
        Err(CredentialRotationError::Identity(
            IdentityError::UnknownIssuer
        ))
    );
}

#[test]
fn signed_revocation_is_terminal_and_advances_trust_revision() {
    let mut record = trusted_record();
    let root_key = SigningKey::from_secret_bytes([4; 32]);
    let root = OwnerRootRecord::new(record.owner_id(), &root_key, 0);
    let transition_id = TransitionId::from_bytes([5; 32]);
    let transition =
        TrustTransition::issue_root_revocation(&record, transition_id, &root, &root_key).unwrap();

    transition.apply_root(&mut record, &root).unwrap();

    assert_eq!(record.state(), TrustState::Revoked);
    assert_eq!(record.trust_revision(), 1);
    assert_eq!(record.last_transition_id(), transition_id);
    assert_eq!(
        TrustTransition::issue_root_revocation(
            &record,
            TransitionId::from_bytes([6; 32]),
            &root,
            &root_key,
        ),
        Err(TrustTransitionError::AlreadyRevoked)
    );
}
