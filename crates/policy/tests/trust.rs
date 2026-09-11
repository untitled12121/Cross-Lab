use crosslab_crypto::SigningKey;
use crosslab_identity::{DeviceId, OwnerId, OwnerRootRecord};
use crosslab_policy::{
    TransitionId, TrustError, TrustRecord, TrustState, TrustTransition, TrustTransitionError,
};

fn trusted_record() -> TrustRecord {
    TrustRecord::trusted(
        OwnerId::from_bytes([1; 32]),
        DeviceId::from_bytes([2; 32]),
        0,
        TransitionId::from_bytes([3; 32]),
    )
}

#[test]
fn trusted_record_starts_at_revision_zero() {
    let record = trusted_record();

    assert_eq!(record.state(), TrustState::Trusted);
    assert_eq!(record.accepted_credential_epoch(), 0);
    assert_eq!(record.trust_revision(), 0);
}

#[test]
fn credential_epoch_advancement_preserves_identity_and_trust() {
    let mut record = trusted_record();
    let owner_id = record.owner_id();
    let device_id = record.device_id();

    record.advance_credential_epoch(1).unwrap();

    assert_eq!(record.owner_id(), owner_id);
    assert_eq!(record.device_id(), device_id);
    assert_eq!(record.state(), TrustState::Trusted);
    assert_eq!(record.accepted_credential_epoch(), 1);
    assert_eq!(record.trust_revision(), 0);
}

#[test]
fn credential_epoch_rejects_stale_and_skipped_values() {
    let mut record = trusted_record();

    assert_eq!(
        record.advance_credential_epoch(0),
        Err(TrustError::StaleCredentialEpoch)
    );
    assert_eq!(
        record.advance_credential_epoch(2),
        Err(TrustError::UnexpectedCredentialEpoch)
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
