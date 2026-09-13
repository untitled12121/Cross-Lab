use crosslab_core::{
    PairingId, PairingInstant, PairingInvitation, PairingInvitationError, PairingInvitationState,
    PairingSecret,
};
use crosslab_identity::{DeviceId, OwnerId};

#[test]
fn pairing_identifiers_and_secret_have_secure_generation_apis() {
    let first_id = PairingId::generate().unwrap();
    let second_id = PairingId::generate().unwrap();
    assert_ne!(first_id, second_id);

    let first_secret = PairingSecret::generate().unwrap();
    let second_secret = PairingSecret::generate().unwrap();
    assert_ne!(first_secret.as_bytes(), second_secret.as_bytes());
}

#[test]
fn invitation_records_local_lifetime_and_expires_at_deadline() {
    let created_at = PairingInstant::from_ticks(10);
    let deadline = PairingInstant::from_ticks(20);
    let mut invitation = PairingInvitation::from_parts(
        PairingId::from_bytes([1; 16]),
        PairingSecret::from_bytes([2; 32]),
        OwnerId::from_bytes([3; 32]),
        DeviceId::from_bytes([4; 32]),
        created_at,
        deadline,
    )
    .unwrap();

    assert_eq!(invitation.created_at(), created_at);
    assert_eq!(invitation.deadline(), deadline);
    assert_eq!(
        invitation.ensure_pending_at(PairingInstant::from_ticks(19)),
        Ok(())
    );
    assert_eq!(invitation.state(), PairingInvitationState::Pending);
    assert_eq!(
        invitation.ensure_pending_at(deadline),
        Err(PairingInvitationError::Expired)
    );
    assert_eq!(invitation.state(), PairingInvitationState::Expired);
    assert_eq!(
        invitation.ensure_pending_at(PairingInstant::from_ticks(21)),
        Err(PairingInvitationError::NotPending)
    );
}

#[test]
fn invitation_rejects_non_forward_deadline() {
    let now = PairingInstant::from_ticks(20);
    let result = PairingInvitation::from_parts(
        PairingId::from_bytes([1; 16]),
        PairingSecret::from_bytes([2; 32]),
        OwnerId::from_bytes([3; 32]),
        DeviceId::from_bytes([4; 32]),
        now,
        now,
    );

    assert_eq!(result, Err(PairingInvitationError::InvalidDeadline));
}
