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
    assert_eq!(
        delegation.signature().to_bytes(),
        [
            107, 234, 244, 210, 203, 95, 220, 252, 211, 183, 10, 139, 93, 40, 134, 187, 146,
            166, 17, 215, 201, 30, 112, 29, 237, 49, 124, 156, 17, 134, 92, 180, 19, 61, 36,
            46, 54, 32, 184, 119, 184, 64, 54, 223, 245, 234, 77, 65, 167, 160, 83, 165, 155,
            222, 120, 114, 89, 4, 37, 219, 252, 108, 110, 7,
        ]
    );
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
    assert_eq!(
        credential.signature().to_bytes(),
        [
            217, 46, 183, 183, 96, 199, 184, 32, 167, 4, 233, 125, 230, 53, 106, 98, 161, 251,
            230, 11, 243, 14, 173, 21, 62, 123, 68, 100, 217, 9, 123, 252, 36, 219, 166, 30,
            78, 23, 222, 105, 26, 141, 242, 95, 27, 39, 73, 54, 144, 235, 152, 113, 22, 186,
            47, 248, 180, 146, 10, 239, 158, 178, 250, 0,
        ]
    );
}
