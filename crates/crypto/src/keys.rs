use core::fmt;

use ed25519_dalek::{Signer, Verifier};
use zeroize::Zeroize;

use crate::{RandomError, random_bytes};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum SignatureAlgorithm {
    Ed25519 = 1,
}

impl SignatureAlgorithm {
    pub const fn code(self) -> u16 {
        self as u16
    }
}

pub struct SigningKey(ed25519_dalek::SigningKey);

impl SigningKey {
    pub fn generate() -> Result<Self, RandomError> {
        let secret = random_bytes::<32>()?;
        Ok(Self::from_secret_bytes(secret))
    }

    pub fn from_secret_bytes(mut secret: [u8; 32]) -> Self {
        let key = Self(ed25519_dalek::SigningKey::from_bytes(&secret));
        secret.zeroize();
        key
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey(self.0.verifying_key())
    }

    pub fn sign_digest(&self, digest: &[u8; 32]) -> Signature {
        Signature(self.0.sign(digest))
    }
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SigningKey([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifyingKey(ed25519_dalek::VerifyingKey);

impl VerifyingKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self, VerificationError> {
        ed25519_dalek::VerifyingKey::from_bytes(&bytes)
            .map(Self)
            .map_err(|_| VerificationError)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.0.to_bytes()
    }

    pub fn verify_digest(
        &self,
        digest: &[u8; 32],
        signature: &Signature,
    ) -> Result<(), VerificationError> {
        self.0
            .verify(digest, &signature.0)
            .map_err(|_| VerificationError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signature(ed25519_dalek::Signature);

impl Signature {
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(ed25519_dalek::Signature::from_bytes(&bytes))
    }

    pub fn to_bytes(self) -> [u8; 64] {
        self.0.to_bytes()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VerificationError;

impl fmt::Debug for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerificationError")
    }
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("signature verification failed")
    }
}

impl std::error::Error for VerificationError {}
