use core::fmt;

use crosslab_crypto::{RandomError, random_bytes};
use zeroize::Zeroize;

mod bootstrap;
mod flow;
mod invitation;
mod transcript;

pub use bootstrap::{
    PAIRING_BOOTSTRAP_PROFILE_V1, PairingBootstrap, PairingBootstrapCode, PairingBootstrapError,
};
pub use flow::{
    PairingFlowError, PairingInviterFlow, PairingInviterState, PairingJoinerFlow,
    PairingJoinerState,
};
pub use invitation::{
    PairingInstant, PairingInvitation, PairingInvitationCreateError, PairingInvitationError,
    PairingInvitationState,
};
pub use transcript::PairingTranscript;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PairingId([u8; 16]);

impl PairingId {
    pub fn generate() -> Result<Self, RandomError> {
        random_bytes().map(Self)
    }

    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

pub struct PairingSecret([u8; 32]);

impl PairingSecret {
    pub fn generate() -> Result<Self, RandomError> {
        random_bytes().map(Self)
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PairingSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingSecret([REDACTED])")
    }
}

impl Drop for PairingSecret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingConfirmationRole {
    Inviter,
    Joiner,
}

impl PairingConfirmationRole {
    const fn label(self) -> &'static [u8] {
        match self {
            Self::Inviter => b"crosslab.pairing.inviter-confirm.v1",
            Self::Joiner => b"crosslab.pairing.joiner-confirm.v1",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PairingConfirmationError;

impl fmt::Debug for PairingConfirmationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingConfirmationError")
    }
}

impl fmt::Display for PairingConfirmationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("pairing confirmation verification failed")
    }
}

impl std::error::Error for PairingConfirmationError {}
