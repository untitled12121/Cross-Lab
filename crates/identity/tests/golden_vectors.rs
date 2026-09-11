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

    assert_eq!(delegation.transcript_digest(), [0; 32]);
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

    assert_eq!(credential.transcript_digest(), [0; 32]);
    assert_eq!(credential.signature().to_bytes(), [0; 64]);
}
