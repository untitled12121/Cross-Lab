use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};

#[test]
fn authority_delegation_v1_golden_vector() {
    let owner_id = OwnerId::from_bytes([0x11; 32]);
    let root_key = SigningKey::from_secret_bytes([0x22; 32]);
    let delegated_key = SigningKey::from_secret_bytes([0x33; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &delegated_key,
        7,
        &root_key,
    );

    assert_eq!(
        delegation.transcript_digest(),
        [
            83, 1, 126, 178, 114, 48, 121, 92, 42, 38, 56, 246, 9, 253, 198, 138, 172, 47, 139,
            250, 138, 5, 1, 248, 42, 239, 185, 255, 75, 238, 185, 106,
        ]
    );
    assert_eq!(delegation.signature().to_bytes(), [0; 64]);
}

#[test]
fn device_credential_v1_golden_vector() {
    let owner_id = OwnerId::from_bytes([0x11; 32]);
    let root_key = SigningKey::from_secret_bytes([0x22; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer_key = SigningKey::from_secret_bytes([0x33; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        7,
        &root_key,
    );
    let device_key = SigningKey::from_secret_bytes([0x55; 32]);
    let credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([0x44; 32]),
        &device_key,
        9,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();

    assert_eq!(
        credential.transcript_digest(),
        [
            207, 25, 126, 225, 229, 238, 172, 144, 202, 155, 222, 239, 248, 43, 13, 116, 169, 33,
            235, 115, 24, 36, 173, 237, 192, 83, 205, 67, 249, 139, 208, 253,
        ]
    );
    assert_eq!(credential.signature().to_bytes(), [0; 64]);
}
