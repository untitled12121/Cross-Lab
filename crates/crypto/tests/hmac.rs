use crosslab_crypto::{HmacVerificationError, hmac_sha256, verify_hmac_sha256};

#[test]
fn hmac_sha256_matches_fixed_vector() {
    let key = b"my secret and secure key";
    let message = b"input message";
    let expected = [
        0x97, 0xd2, 0xa5, 0x69, 0x05, 0x9b, 0xbc, 0xd8, 0xea, 0xd4, 0x44, 0x4f, 0xf9, 0x90, 0x71,
        0xf4, 0xc0, 0x1d, 0x00, 0x5b, 0xce, 0xfe, 0x0d, 0x35, 0x67, 0xe1, 0xbe, 0x62, 0x8e, 0x5f,
        0xdc, 0xd9,
    ];

    assert_eq!(hmac_sha256(key, message), expected);
    assert_eq!(verify_hmac_sha256(key, message, &expected), Ok(()));
}

#[test]
fn hmac_sha256_verification_rejects_tampering() {
    let key = [0x42; 32];
    let message = b"crosslab pairing confirmation";
    let mut tag = hmac_sha256(&key, message);
    tag[0] ^= 0x80;

    assert_eq!(
        verify_hmac_sha256(&key, message, &tag),
        Err(HmacVerificationError)
    );
}
