use crosslab_crypto::{SignatureAlgorithm, SigningKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};

#[test]
fn signed_device_credential_can_be_reconstructed_without_private_keys() {
    let owner_id = OwnerId::from_bytes([0x11; 32]);
    let root_key = SigningKey::from_secret_bytes([0x12; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 3);
    let issuer_key = SigningKey::from_secret_bytes([0x13; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        4,
        &root_key,
    );
    let device_key = SigningKey::from_secret_bytes([0x14; 32]);
    let credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([0x15; 32]),
        &device_key,
        7,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();

    let imported = DeviceCredential::from_signed_parts(
        1,
        credential.owner_id(),
        credential.device_id(),
        credential.device_key_id(),
        SignatureAlgorithm::Ed25519,
        credential.device_public_key(),
        credential.credential_epoch(),
        credential.issuer_device_signing_key_id(),
        credential.signature(),
    )
    .unwrap();

    assert_eq!(imported, credential);
    imported.verify(&root, &delegation, 7, 4).unwrap();
}
