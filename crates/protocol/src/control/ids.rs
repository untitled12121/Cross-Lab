use core::fmt;

use crosslab_crypto::random_bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolIdError;

impl fmt::Display for ProtocolIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("secure protocol identifier generation failed")
    }
}

impl std::error::Error for ProtocolIdError {}

macro_rules! protocol_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 16]);

        impl $name {
            pub const fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(bytes)
            }

            pub const fn to_bytes(self) -> [u8; 16] {
                self.0
            }

            pub fn generate() -> Result<Self, ProtocolIdError> {
                random_bytes::<16>().map(Self).map_err(|_| ProtocolIdError)
            }
        }
    };
}

protocol_id!(RequestId);
protocol_id!(EventId);
protocol_id!(StreamId);
