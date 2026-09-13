use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    CredentialRotationError, TransitionId, TrustRecord, TrustState, TrustTransition,
    TrustTransitionError,
};

fn trusted_record() -> TrustRecord {
    TrustRecord::trusted(
        OwnerId::from_bytes([1; 32]),
        DeviceId::from_bytes([2; 32]),
        0,
        TransitionId::from_bytes([3; 32]),
    )
}

struct RotationFixture {
    owner_id: OwnerId,
    device_id: DeviceId,
    root: OwnerRootRecord,
    issuer_key: SigningKey,
    delegation: AuthorityDelegation,
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

        Self {
            owner_id,
            device_id: DeviceId::from_bytes([0x23; 32]),
            root,
            issuer_key,
            delegation,
        }
    }

    fn trust(&self) -> TrustRecord {
        TrustRecord::trusted(
            self.owner_id,
            self.device_id,
            0,
            TransitionId::from_bytes([0x24; 32]),
        )
    }

    fn credential(&self, device_id: DeviceId, epoch: u64) -> DeviceCredential {
        DeviceCredential::issue(
            self.owner_id,
            device_id,
            &SigningKey::from_secret_bytes([0x25; 32]),
            epoch,
            &self.root,
            &self.delegation,
            &self.issuer_key,
        )
        .unwrap()
    }
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
        .accept_successor_credential(
            &successor,
            &fixture.root,
            &fixture.delegation,
            fixture.delegation.delegation_epoch(),
            transition_id,
        )
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
        stale_record.accept_successor_credential(
            &stale,
            &fixture.root,
            &fixture.delegation,
            fixture.delegation.delegation_epoch(),
            TransitionId::from_bytes([0x27; 32]),
        ),
        Err(CredentialRotationError::StaleCredentialEpoch)
    );

    let mut skipped_record = fixture.trust();
    let skipped = fixture.credential(fixture.device_id, 2);
    assert_eq!(
        skipped_record.accept_successor_credential(
            &skipped,
            &fixture.root,
            &fixture.delegation,
            fixture.delegation.delegation_epoch(),
            TransitionId::from_bytes([0x28; 32]),
        ),
        Err(CredentialRotationError::UnexpectedCredentialEpoch)
    );

    let mut wrong_device_record = fixture.trust();
    let wrong_device = fixture.credential(DeviceId::from_bytes([0x29; 32]), 1);
    assert_eq!(
        wrong_device_record.accept_successor_credential(
            &wrong_device,
            &fixture.root,
            &fixture.delegation,
            fixture.delegation.delegation_epoch(),
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
