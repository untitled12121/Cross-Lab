use crosslab_identity::{DeviceId, OwnerId};
use crosslab_policy::{TransitionId, TrustError, TrustRecord, TrustState};

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
fn revocation_is_terminal_and_advances_trust_revision() {
    let mut record = trusted_record();
    let transition = TransitionId::from_bytes([4; 32]);

    record.revoke(transition).unwrap();

    assert_eq!(record.state(), TrustState::Revoked);
    assert_eq!(record.trust_revision(), 1);
    assert_eq!(record.last_transition_id(), transition);
    assert_eq!(
        record.revoke(TransitionId::from_bytes([5; 32])),
        Err(TrustError::AlreadyRevoked)
    );
}
