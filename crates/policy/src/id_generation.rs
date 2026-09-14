use core::fmt;

use crosslab_crypto::random_bytes;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PolicyIdGenerationError;

impl fmt::Debug for PolicyIdGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PolicyIdGenerationError")
    }
}

impl fmt::Display for PolicyIdGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("secure policy identifier generation failed")
    }
}

impl std::error::Error for PolicyIdGenerationError {}

pub(crate) fn random_policy_id() -> Result<[u8; 32], PolicyIdGenerationError> {
    random_bytes().map_err(|_| PolicyIdGenerationError)
}
