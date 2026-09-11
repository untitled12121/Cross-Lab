use core::fmt;

use crosslab_crypto::{SignatureAlgorithm, VerifyingKey, blake3_256, random_bytes};

const KEY_ID_DOMAIN: &[u8] = b"crosslab.key-id.v1";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OwnerId([u8; 32]);

impl OwnerId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn generate() -> Result<Self, IdGenerationError> {
        random_bytes().map(Self).map_err(|_| IdGenerationError)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for OwnerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("OwnerId").field(&self.0).finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId([u8; 32]);

impl DeviceId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn generate() -> Result<Self, IdGenerationError> {
        random_bytes().map(Self).map_err(|_| IdGenerationError)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for DeviceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("DeviceId").field(&self.0).finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyId([u8; 32]);

impl KeyId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn derive(algorithm: SignatureAlgorithm, public_key: &VerifyingKey) -> Self {
        let mut input = Vec::with_capacity(KEY_ID_DOMAIN.len() + 2 + 32);
        input.extend_from_slice(KEY_ID_DOMAIN);
        input.extend_from_slice(&algorithm.code().to_be_bytes());
        input.extend_from_slice(public_key.as_bytes());
        Self(blake3_256(&input))
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for KeyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("KeyId").field(&self.0).finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IdGenerationError;

impl fmt::Debug for IdGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IdGenerationError")
    }
}

impl fmt::Display for IdGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("failed to generate a stable identity identifier")
    }
}

impl std::error::Error for IdGenerationError {}
