use crosslab_crypto::{SignatureAlgorithm, SigningKey, blake3_256};
use crosslab_identity::{DeviceId, KeyId, OwnerId};

#[test]
fn owner_and_device_ids_preserve_exact_32_byte_values() {
    let owner_bytes = [0x11; 32];
    let device_bytes = [0x22; 32];

    let owner = OwnerId::from_bytes(owner_bytes);
    let device = DeviceId::from_bytes(device_bytes);

    assert_eq!(owner.to_bytes(), owner_bytes);
    assert_eq!(device.to_bytes(), device_bytes);
}

#[test]
fn generated_ids_use_the_full_256_bit_domain_shape() {
    assert_eq!(OwnerId::generate().unwrap().as_bytes().len(), 32);
    assert_eq!(DeviceId::generate().unwrap().as_bytes().len(), 32);
}

#[test]
fn key_id_matches_the_v1_domain_separated_definition() {
    let signing_key = SigningKey::from_secret_bytes([3_u8; 32]);
    let verifying_key = signing_key.verifying_key();

    let key_id = KeyId::derive(SignatureAlgorithm::Ed25519, &verifying_key);

    let mut input = Vec::new();
    input.extend_from_slice(b"crosslab.key-id.v1");
    input.extend_from_slice(&1_u16.to_be_bytes());
    input.extend_from_slice(verifying_key.as_bytes());

    assert_eq!(key_id.to_bytes(), blake3_256(&input));
}
