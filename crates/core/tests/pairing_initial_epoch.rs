use crosslab_core::{
    PairingId, PairingInvitation, PairingInviterFlow, PairingJoinerFlow, PairingSecret,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{AuthorityDelegation, AuthorityRole, DeviceId, OwnerId, OwnerRootRecord};
use crosslab_protocol::{PairingHello, PairingRole};

#[test]
fn initial_pairing_credential_epoch_is_zero() {
    let owner_id = OwnerId::from_bytes([0x41; 32]);
    let root_key = SigningKey::from_secret_bytes([0x42; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer_key = SigningKey::from_secret_bytes([0x43; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        0,
        &root_key,
    );
    let inviter_key = SigningKey::from_secret_bytes([0x44; 32]);
    let joiner_key = SigningKey::from_secret_bytes([0x45; 32]);
    let inviter_device_id = DeviceId::from_bytes([0x46; 32]);
    let joiner_device_id = DeviceId::from_bytes([0x47; 32]);
    let pairing_id = PairingId::from_bytes([0x48; 16]);
    let secret = [0x49; 32];

    let inviter_hello = PairingHello::new(
        PairingRole::Inviter,
        1,
        pairing_id.to_bytes(),
        owner_id,
        inviter_device_id,
        inviter_key.verifying_key(),
        [0x4a; 32],
    );
    let joiner_hello = PairingHello::new(
        PairingRole::Joiner,
        1,
        pairing_id.to_bytes(),
        owner_id,
        joiner_device_id,
        joiner_key.verifying_key(),
        [0x4b; 32],
    );
    let invitation = PairingInvitation::from_parts(
        pairing_id,
        PairingSecret::from_bytes(secret),
        owner_id,
        inviter_device_id,
    );
    let mut inviter = PairingInviterFlow::new(invitation, inviter_hello, joiner_hello).unwrap();
    let joiner = PairingJoinerFlow::new(
        PairingSecret::from_bytes(secret),
        inviter_hello,
        joiner_hello,
    )
    .unwrap();

    let confirmation = joiner.joiner_confirmation().unwrap();
    inviter.verify_joiner_confirmation(&confirmation).unwrap();

    let credential = inviter
        .issue_joiner_credential(&root, &delegation, &issuer_key, 9)
        .unwrap();

    assert_eq!(credential.credential_epoch(), 0);
}
