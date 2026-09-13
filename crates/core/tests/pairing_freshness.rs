use crosslab_core::{
    PairingConfirmationRole, PairingId, PairingInstant, PairingInvitation, PairingInvitationError,
    PairingInvitationState, PairingSecret, PairingTranscript,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{DeviceId, OwnerId};

fn transcript() -> PairingTranscript {
    PairingTranscript::new(
        1,
        PairingId::from_bytes([0x31; 16]),
        OwnerId::from_bytes([0x32; 32]),
        DeviceId::from_bytes([0x33; 32]),
        SigningKey::from_secret_bytes([0x34; 32]).verifying_key(),
        [0x35; 32],
        DeviceId::from_bytes([0x36; 32]),
        SigningKey::from_secret_bytes([0x37; 32]).verifying_key(),
        [0x38; 32],
    )
}

#[test]
fn pairing_identifiers_and_secret_have_secure_generation_apis() {
    let first_id = PairingId::generate().unwrap();
    let second_id = PairingId::generate().unwrap();
    assert_ne!(first_id, second_id);

    let first_secret = PairingSecret::generate().unwrap();
    let second_secret = PairingSecret::generate().unwrap();
    let transcript = transcript();
    assert_ne!(
        transcript.confirmation(PairingConfirmationRole::Inviter, &first_secret),
        transcript.confirmation(PairingConfirmationRole::Inviter, &second_secret)
    );
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

    assert!(matches!(
        result,
        Err(PairingInvitationError::InvalidDeadline)
    ));
}
