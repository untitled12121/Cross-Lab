use core::fmt;

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC-SHA-256 accepts any key length");
    mac.update(message);

    let bytes = mac.finalize().into_bytes();
    let mut tag = [0_u8; 32];
    tag.copy_from_slice(&bytes);
    tag
}

pub fn verify_hmac_sha256(
    key: &[u8],
    message: &[u8],
    expected: &[u8; 32],
) -> Result<(), HmacVerificationError> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC-SHA-256 accepts any key length");
    mac.update(message);
    mac.verify_slice(expected).map_err(|_| HmacVerificationError)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HmacVerificationError;

impl fmt::Debug for HmacVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HmacVerificationError")
    }
}

impl fmt::Display for HmacVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HMAC-SHA-256 verification failed")
    }
}

impl std::error::Error for HmacVerificationError {}
