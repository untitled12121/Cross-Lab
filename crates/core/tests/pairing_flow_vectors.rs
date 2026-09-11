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
        [23; 16],
        [
            130, 248, 137, 98, 89, 1, 150, 41, 145, 57, 188, 215, 171, 216, 190, 92, 202,
            255, 98, 250, 218, 254, 75, 213, 45, 92, 48, 50, 153, 149, 182, 173,
        ],
        [
            56, 129, 41, 36, 219, 204, 136, 6, 120, 146, 179, 209, 96, 77, 73, 158, 165,
            118, 116, 154, 9, 24, 23, 172, 64, 124, 26, 12, 53, 91, 5, 197,
        ],
        [22; 32],
        [
            70, 138, 146, 184, 16, 146, 212, 59, 205, 168, 165, 95, 99, 64, 147, 160, 63,
            138, 3, 35, 55, 127, 184, 214, 157, 4, 67, 6, 79, 95, 21, 92,
        ],
        [
            64, 130, 1, 142, 71, 94, 41, 107, 176, 171, 178, 77, 77, 251, 202, 98, 88,
            131, 165, 228, 109, 90, 104, 107, 16, 253, 74, 236, 195, 115, 164, 159, 0,
            191, 35, 49, 36, 115, 156, 41, 218, 12, 114, 36, 73, 90, 250, 90, 5, 52, 3,
            114, 216, 161, 243, 146, 253, 129, 255, 18, 203, 13, 189, 15,
        ],
    );

    assert_eq!(actual, expected);
}
