use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};

#[test]
fn device_credential_can_be_issued_from_public_key_without_device_private_key() {
    let owner_id = OwnerId::from_bytes([0x51; 32]);
    let root_key = SigningKey::from_secret_bytes([0x52; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer_key = SigningKey::from_secret_bytes([0x53; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        0,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();
    let device_key = SigningKey::from_secret_bytes([0x54; 32]);
    let device_public_key = device_key.verifying_key();
    let device_id = DeviceId::from_bytes([0x55; 32]);

    let credential = DeviceCredential::issue_for_public_key_current(
        owner_id,
        device_id,
        device_public_key,
        3,
        &authority,
        &issuer_key,
    )
    .unwrap();

    credential.verify_current(&authority, 3).unwrap();
    assert_eq!(credential.device_id(), device_id);
    assert_eq!(credential.device_public_key(), device_public_key);
    assert_eq!(credential.credential_epoch(), 3);
}
