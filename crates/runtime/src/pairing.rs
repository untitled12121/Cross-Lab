use core::fmt;

use crosslab_core::PairingFlowError;
use crosslab_identity::{AuthorityDelegation, DeviceCredential, IdentityError, OwnerRootRecord};
use crosslab_policy::{PairingTrustTransition, TrustRecord};

const PROTOCOL_MAJOR_V1: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPairingState {
    AwaitingPeerHello,
    AwaitingPeerConfirmation,
    AwaitingCredential,
    AwaitingCredentialAcceptance,
    AwaitingPeerTrust,
    AwaitingPersistence,
    Complete,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductPairingCommit {
    peer_credential: DeviceCredential,
    peer_transition: PairingTrustTransition,
    peer_trust: TrustRecord,
}

impl ProductPairingCommit {
    pub const fn peer_credential(&self) -> DeviceCredential {
        self.peer_credential
    }

    pub const fn peer_transition(&self) -> PairingTrustTransition {
        self.peer_transition
    }

    pub const fn peer_trust(&self) -> TrustRecord {
        self.peer_trust
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductPairingTrustBundle {
    credential: DeviceCredential,
    transition: PairingTrustTransition,
}

impl ProductPairingTrustBundle {
    pub const fn credential(&self) -> DeviceCredential {
        self.credential
    }

    pub const fn transition(&self) -> PairingTrustTransition {
        self.transition
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductPairingInviterCompletion {
    commit: ProductPairingCommit,
    reciprocal_trust: ProductPairingTrustBundle,
}

impl ProductPairingInviterCompletion {
    pub const fn commit(&self) -> ProductPairingCommit {
        self.commit
    }

    pub const fn reciprocal_trust(&self) -> ProductPairingTrustBundle {
        self.reciprocal_trust
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductPairingJoinerCompletion {
    owner_root: OwnerRootRecord,
    device_signing: AuthorityDelegation,
    local_credential: DeviceCredential,
    peer_commit: ProductPairingCommit,
}

impl ProductPairingJoinerCompletion {
    pub const fn owner_root(&self) -> OwnerRootRecord {
        self.owner_root
    }

    pub const fn device_signing(&self) -> AuthorityDelegation {
        self.device_signing
    }

    pub const fn local_credential(&self) -> DeviceCredential {
        self.local_credential
    }

    pub const fn peer_commit(&self) -> ProductPairingCommit {
        self.peer_commit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPairingError {
    Random,
    InvalidState,
    LocalIdentityMismatch,
    BootstrapPeerMismatch,
    Identity(IdentityError),
    Flow(PairingFlowError),
}

impl fmt::Display for ProductPairingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Random => formatter.write_str("secure pairing randomness is unavailable"),
            Self::InvalidState => {
                formatter.write_str("product pairing operation is out of sequence")
            }
            Self::LocalIdentityMismatch => {
                formatter.write_str("local product identity does not match the pairing invitation")
            }
            Self::BootstrapPeerMismatch => {
                formatter.write_str("pairing peer does not match the scanned bootstrap")
            }
            Self::Identity(error) => fmt::Display::fmt(error, formatter),
            Self::Flow(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for ProductPairingError {}

impl From<IdentityError> for ProductPairingError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}

impl From<PairingFlowError> for ProductPairingError {
    fn from(error: PairingFlowError) -> Self {
        Self::Flow(error)
    }
}

mod inviter;
mod joiner;

pub use inviter::ProductPairingInviter;
pub use joiner::ProductPairingJoiner;

#[cfg(test)]
mod tests;
