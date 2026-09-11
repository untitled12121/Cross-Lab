use crosslab_core::{
    PairingId, PairingInvitation, PairingInviterFlow, PairingJoinerFlow, PairingSecret,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{AuthorityDelegation, AuthorityRole, DeviceId, OwnerId, OwnerRootRecord};
use crosslab_protocol::{PairingHello, PairingRole};

#[test]
fn credential_acceptance_proof_matches_golden_vector() {
    let owner_id = OwnerId::from_bytes([0x10; 32]);
    let root_key = SigningKey::from_secret_bytes([0x11; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer_key = SigningKey::from_secret_bytes([0x12; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        0,
        &root_key,
    );
    let inviter_key = SigningKey::from_secret_bytes([0x13; 32]);
    let joiner_key = SigningKey::from_secret_bytes([0x14; 32]);
    let inviter_device_id = DeviceId::from_bytes([0x15; 32]);
    let joiner_device_id = DeviceId::from_bytes([0x16; 32]);
    let pairing_id = PairingId::from_bytes([0x17; 16]);
    let secret = [0x18; 32];
    let inviter_hello = PairingHello::new(
        PairingRole::Inviter,
        1,
        pairing_id.to_bytes(),
        owner_id,
        inviter_device_id,
        inviter_key.verifying_key(),
        [0x19; 32],
    );
    let joiner_hello = PairingHello::new(
        PairingRole::Joiner,
        1,
        pairing_id.to_bytes(),
        owner_id,
        joiner_device_id,
        joiner_key.verifying_key(),
        [0x1a; 32],
    );
    let invitation = PairingInvitation::from_parts(
        pairing_id,
        PairingSecret::from_bytes(secret),
        owner_id,
        inviter_device_id,
    );
    let mut inviter = PairingInviterFlow::new(invitation, inviter_hello, joiner_hello).unwrap();
    let mut joiner = PairingJoinerFlow::new(
        PairingSecret::from_bytes(secret),
        inviter_hello,
        joiner_hello,
    )
    .unwrap();
    let joiner_confirmation = joiner.joiner_confirmation().unwrap();
    let inviter_confirmation = inviter
        .verify_joiner_confirmation(&joiner_confirmation)
        .unwrap();
    joiner
        .verify_inviter_confirmation(&inviter_confirmation)
        .unwrap();
    let credential = inviter
        .issue_joiner_credential(&root, &delegation, &issuer_key, 7)
        .unwrap();
    let accepted = joiner
        .accept_credential(&root, &delegation, &credential, &joiner_key)
        .unwrap();

    let actual = (
        accepted.pairing_id(),
        accepted.pairing_transcript_digest(),
        accepted.device_credential_signed_object_digest(),
        accepted.joiner_device_id().to_bytes(),
        accepted.joiner_device_key_id().to_bytes(),
        accepted.signature().to_bytes(),
    );
    let expected = (
        [0_u8; 16], [0_u8; 32], [0_u8; 32], [0_u8; 32], [0_u8; 32], [0_u8; 64],
    );

    assert_eq!(actual, expected);
}
