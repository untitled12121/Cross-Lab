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
        .issue_initial_joiner_credential(&root, &delegation, &issuer_key)
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
        [23; 16],
        [
            130, 248, 137, 98, 89, 1, 150, 41, 145, 57, 188, 215, 171, 216, 190, 92, 202, 255, 98,
            250, 218, 254, 75, 213, 45, 92, 48, 50, 153, 149, 182, 173,
        ],
        [
            227, 146, 182, 38, 241, 247, 224, 44, 4, 8, 150, 174, 183, 174, 139, 27, 115, 220, 158,
            6, 66, 66, 190, 234, 236, 70, 226, 171, 9, 7, 104, 185,
        ],
        [22; 32],
        [
            70, 138, 146, 184, 16, 146, 212, 59, 205, 168, 165, 95, 99, 64, 147, 160, 63, 138, 3,
            35, 55, 127, 184, 214, 157, 4, 67, 6, 79, 95, 21, 92,
        ],
        [
            123, 199, 44, 128, 53, 10, 205, 2, 177, 127, 123, 203, 254, 173, 40, 175, 28, 42, 163,
            190, 124, 169, 146, 46, 97, 56, 80, 246, 137, 201, 36, 26, 95, 42, 249, 47, 212, 28,
            61, 229, 52, 133, 184, 23, 110, 137, 192, 120, 0, 104, 88, 181, 90, 125, 150, 27, 54,
            200, 144, 92, 230, 156, 246, 3,
        ],
    );

    assert_eq!(actual, expected);
}
