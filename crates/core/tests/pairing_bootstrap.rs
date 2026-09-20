use crosslab_core::{
    PairingBootstrap, PairingBootstrapError, PairingId, PairingInstant, PairingInvitation,
    PairingInvitationError, PairingInvitationState, PairingInviterFlow, PairingJoinerFlow,
    PairingSecret,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{DeviceId, OwnerId};
use crosslab_protocol::{PairingHello, PairingRole};

const PREFIX: &str = "crosslab:pair:v1:";

fn invitation() -> PairingInvitation {
    PairingInvitation::from_parts(
        PairingId::from_bytes([0x11; 16]),
        PairingSecret::from_bytes([0x22; 32]),
        OwnerId::from_bytes([0x33; 32]),
        DeviceId::from_bytes([0x44; 32]),
        PairingInstant::from_ticks(0),
        PairingInstant::from_ticks(100),
    )
    .unwrap()
}

#[test]
fn bootstrap_round_trip_authenticates_the_existing_pairing_flow() {
    let mut invitation = invitation();
    let code = invitation
        .bootstrap_code_at(PairingInstant::from_ticks(10))
        .unwrap();

    assert!(code.as_str().starts_with(PREFIX));
    assert_eq!(code.as_str().len(), PREFIX.len() + 114 * 2);
    assert_eq!(format!("{code:?}"), "PairingBootstrapCode([REDACTED])");

    let bootstrap = PairingBootstrap::decode(code.as_str()).unwrap();
    assert_eq!(bootstrap.pairing_id(), PairingId::from_bytes([0x11; 16]));
    assert_eq!(bootstrap.owner_id(), OwnerId::from_bytes([0x33; 32]));
    assert_eq!(
        bootstrap.inviter_device_id(),
        DeviceId::from_bytes([0x44; 32])
    );
    assert!(!format!("{bootstrap:?}").contains(&"22".repeat(32)));

    let (pairing_id, secret, owner_id, inviter_device_id) = bootstrap.into_parts();
    let inviter_key = SigningKey::from_secret_bytes([0x55; 32]);
    let joiner_key = SigningKey::from_secret_bytes([0x66; 32]);
    let inviter = PairingHello::new(
        PairingRole::Inviter,
        1,
        pairing_id.to_bytes(),
        owner_id,
        inviter_device_id,
        inviter_key.verifying_key(),
        [0x77; 32],
    );
    let joiner = PairingHello::new(
        PairingRole::Joiner,
        1,
        pairing_id.to_bytes(),
        owner_id,
        DeviceId::from_bytes([0x88; 32]),
        joiner_key.verifying_key(),
        [0x99; 32],
    );

    let mut inviter_flow =
        PairingInviterFlow::new(invitation, inviter, joiner, PairingInstant::from_ticks(10))
            .unwrap();
    let joiner_flow = PairingJoinerFlow::new(secret, inviter, joiner).unwrap();
    let confirmation = joiner_flow.joiner_confirmation().unwrap();

    inviter_flow
        .verify_joiner_confirmation(&confirmation, PairingInstant::from_ticks(20))
        .unwrap();
}

#[test]
fn bootstrap_code_requires_a_current_pending_invitation() {
    let mut cancelled = invitation();
    cancelled.cancel().unwrap();
    assert!(matches!(
        cancelled.bootstrap_code_at(PairingInstant::from_ticks(10)),
        Err(PairingInvitationError::NotPending)
    ));

    let mut expired = invitation();
    assert!(matches!(
        expired.bootstrap_code_at(PairingInstant::from_ticks(100)),
        Err(PairingInvitationError::Expired)
    ));
    assert_eq!(expired.state(), PairingInvitationState::Expired);
}

#[test]
fn bootstrap_decoder_rejects_wrong_prefix_length_encoding_and_profile() {
    assert!(matches!(
        PairingBootstrap::decode("https://example.invalid/pair"),
        Err(PairingBootstrapError::InvalidPrefix)
    ));
    assert!(matches!(
        PairingBootstrap::decode(PREFIX),
        Err(PairingBootstrapError::InvalidLength)
    ));

    let mut invitation = invitation();
    let code = invitation
        .bootstrap_code_at(PairingInstant::from_ticks(1))
        .unwrap();

    let mut invalid_encoding = code.as_str().to_owned();
    invalid_encoding.replace_range(PREFIX.len()..PREFIX.len() + 2, "zz");
    assert!(matches!(
        PairingBootstrap::decode(&invalid_encoding),
        Err(PairingBootstrapError::InvalidEncoding)
    ));

    let mut unsupported = code.as_str().to_owned();
    unsupported.replace_range(PREFIX.len()..PREFIX.len() + 4, "0002");
    assert!(matches!(
        PairingBootstrap::decode(&unsupported),
        Err(PairingBootstrapError::UnsupportedProfile)
    ));
}
