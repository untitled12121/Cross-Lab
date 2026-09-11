use crosslab_core::{
    PairingConfirmationRole, PairingId, PairingInvitation, PairingInvitationError,
    PairingInvitationState, PairingSecret, PairingTranscript,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{DeviceId, OwnerId};

fn transcript() -> PairingTranscript {
    let inviter_key = SigningKey::from_secret_bytes([0x11; 32]);
    let joiner_key = SigningKey::from_secret_bytes([0x22; 32]);

    PairingTranscript::new(
        1,
        PairingId::from_bytes([0x33; 16]),
        OwnerId::from_bytes([0x44; 32]),
        DeviceId::from_bytes([0x55; 32]),
        inviter_key.verifying_key(),
        [0x66; 32],
        DeviceId::from_bytes([0x77; 32]),
        joiner_key.verifying_key(),
        [0x88; 32],
    )
}

#[test]
fn pairing_confirmation_is_role_and_transcript_bound() {
    let transcript = transcript();
    let secret = PairingSecret::from_bytes([0x99; 32]);

    let inviter = transcript.confirmation(PairingConfirmationRole::Inviter, &secret);
    let joiner = transcript.confirmation(PairingConfirmationRole::Joiner, &secret);

    assert_ne!(inviter, joiner);
    assert!(
        transcript
            .verify_confirmation(PairingConfirmationRole::Inviter, &secret, &inviter)
            .is_ok()
    );
    assert!(
        transcript
            .verify_confirmation(PairingConfirmationRole::Joiner, &secret, &inviter)
            .is_err()
    );

    let wrong_secret = PairingSecret::from_bytes([0xaa; 32]);
    assert!(
        transcript
            .verify_confirmation(PairingConfirmationRole::Inviter, &wrong_secret, &inviter,)
            .is_err()
    );
}

#[test]
fn pairing_confirmation_rejects_nonce_substitution() {
    let transcript = transcript();
    let secret = PairingSecret::from_bytes([0x99; 32]);
    let confirmation = transcript.confirmation(PairingConfirmationRole::Joiner, &secret);

    let substituted = PairingTranscript::new(
        1,
        PairingId::from_bytes([0x33; 16]),
        OwnerId::from_bytes([0x44; 32]),
        DeviceId::from_bytes([0x55; 32]),
        SigningKey::from_secret_bytes([0x11; 32]).verifying_key(),
        [0x66; 32],
        DeviceId::from_bytes([0x77; 32]),
        SigningKey::from_secret_bytes([0x22; 32]).verifying_key(),
        [0x89; 32],
    );

    assert!(
        substituted
            .verify_confirmation(PairingConfirmationRole::Joiner, &secret, &confirmation)
            .is_err()
    );
}

#[test]
fn pairing_invitation_is_single_use() {
    let mut invitation = PairingInvitation::from_parts(
        PairingId::from_bytes([1; 16]),
        PairingSecret::from_bytes([2; 32]),
        OwnerId::from_bytes([3; 32]),
        DeviceId::from_bytes([4; 32]),
    );

    assert_eq!(invitation.state(), PairingInvitationState::Pending);
    assert_eq!(invitation.consume(), Ok(()));
    assert_eq!(invitation.state(), PairingInvitationState::Consumed);
    assert_eq!(invitation.cancel(), Err(PairingInvitationError::NotPending));
    assert_eq!(invitation.expire(), Err(PairingInvitationError::NotPending));
}

#[test]
fn pairing_secret_debug_output_is_redacted() {
    let secret = PairingSecret::from_bytes([0xab; 32]);
    let debug = format!("{secret:?}");

    assert_eq!(debug, "PairingSecret([REDACTED])");
    assert!(!debug.contains("171"));
}
