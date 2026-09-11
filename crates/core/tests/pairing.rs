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
fn pairing_transcript_equality_is_field_exact() {
    assert_eq!(transcript(), transcript());

    let changed = PairingTranscript::new(
        1,
        PairingId::from_bytes([0x34; 16]),
        OwnerId::from_bytes([0x44; 32]),
        DeviceId::from_bytes([0x55; 32]),
        SigningKey::from_secret_bytes([0x11; 32]).verifying_key(),
        [0x66; 32],
        DeviceId::from_bytes([0x77; 32]),
        SigningKey::from_secret_bytes([0x22; 32]).verifying_key(),
        [0x88; 32],
    );

    assert_ne!(transcript(), changed);
}

#[test]
fn pairing_transcript_and_confirmations_match_golden_vectors() {
    let transcript = transcript();
    let secret = PairingSecret::from_bytes([0x99; 32]);
    let expected_digest = [
        82, 75, 39, 12, 199, 179, 200, 134, 244, 96, 246, 128, 98, 140, 4, 181, 53, 77, 100, 250,
        11, 140, 110, 180, 76, 36, 251, 96, 14, 52, 207, 70,
    ];
    let expected_inviter = [
        91, 2, 96, 118, 92, 175, 212, 42, 72, 105, 132, 77, 137, 100, 56, 26, 92, 134, 225, 141,
        218, 80, 142, 169, 11, 93, 191, 31, 247, 194, 100, 157,
    ];
    let expected_joiner = [
        239, 15, 217, 233, 254, 95, 11, 40, 230, 182, 248, 57, 199, 64, 114, 64, 106, 35, 100, 224,
        43, 178, 222, 187, 62, 32, 154, 161, 235, 132, 187, 217,
    ];

    assert_eq!(transcript.digest(), expected_digest);
    assert_eq!(
        transcript.confirmation(PairingConfirmationRole::Inviter, &secret),
        expected_inviter
    );
    assert_eq!(
        transcript.confirmation(PairingConfirmationRole::Joiner, &secret),
        expected_joiner
    );
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
fn pairing_confirmation_rejects_security_field_substitution() {
    let transcript = transcript();
    let secret = PairingSecret::from_bytes([0x99; 32]);
    let confirmation = transcript.confirmation(PairingConfirmationRole::Joiner, &secret);

    let wrong_pairing_id = PairingTranscript::new(
        1,
        PairingId::from_bytes([0x34; 16]),
        OwnerId::from_bytes([0x44; 32]),
        DeviceId::from_bytes([0x55; 32]),
        SigningKey::from_secret_bytes([0x11; 32]).verifying_key(),
        [0x66; 32],
        DeviceId::from_bytes([0x77; 32]),
        SigningKey::from_secret_bytes([0x22; 32]).verifying_key(),
        [0x88; 32],
    );
    assert!(
        wrong_pairing_id
            .verify_confirmation(PairingConfirmationRole::Joiner, &secret, &confirmation)
            .is_err()
    );

    let wrong_joiner_key = PairingTranscript::new(
        1,
        PairingId::from_bytes([0x33; 16]),
        OwnerId::from_bytes([0x44; 32]),
        DeviceId::from_bytes([0x55; 32]),
        SigningKey::from_secret_bytes([0x11; 32]).verifying_key(),
        [0x66; 32],
        DeviceId::from_bytes([0x77; 32]),
        SigningKey::from_secret_bytes([0x23; 32]).verifying_key(),
        [0x88; 32],
    );
    assert!(
        wrong_joiner_key
            .verify_confirmation(PairingConfirmationRole::Joiner, &secret, &confirmation)
            .is_err()
    );

    let wrong_joiner_nonce = PairingTranscript::new(
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
        wrong_joiner_nonce
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
fn pairing_invitation_cancellation_and_expiry_are_terminal() {
    let mut cancelled = PairingInvitation::from_parts(
        PairingId::from_bytes([1; 16]),
        PairingSecret::from_bytes([2; 32]),
        OwnerId::from_bytes([3; 32]),
        DeviceId::from_bytes([4; 32]),
    );
    assert_eq!(cancelled.cancel(), Ok(()));
    assert_eq!(cancelled.state(), PairingInvitationState::Cancelled);
    assert_eq!(cancelled.consume(), Err(PairingInvitationError::NotPending));

    let mut expired = PairingInvitation::from_parts(
        PairingId::from_bytes([5; 16]),
        PairingSecret::from_bytes([6; 32]),
        OwnerId::from_bytes([7; 32]),
        DeviceId::from_bytes([8; 32]),
    );
    assert_eq!(expired.expire(), Ok(()));
    assert_eq!(expired.state(), PairingInvitationState::Expired);
    assert_eq!(expired.consume(), Err(PairingInvitationError::NotPending));
}

#[test]
fn pairing_secret_debug_output_is_redacted() {
    let secret = PairingSecret::from_bytes([0xab; 32]);
    let debug = format!("{secret:?}");

    assert_eq!(debug, "PairingSecret([REDACTED])");
    assert!(!debug.contains("171"));
}
