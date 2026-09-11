use core::fmt;

use crosslab_identity::{DeviceId, OwnerId};

mod transition;

pub use transition::{TrustTransition, TrustTransitionError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionId([u8; 32]);

impl TransitionId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustState {
    Pending,
    Trusted,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustError {
    StaleCredentialEpoch,
    UnexpectedCredentialEpoch,
    AlreadyRevoked,
}

impl fmt::Display for TrustError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StaleCredentialEpoch => "credential epoch is stale",
            Self::UnexpectedCredentialEpoch => "credential epoch transition is invalid",
            Self::AlreadyRevoked => "device trust is already revoked",
        })
    }
}

impl std::error::Error for TrustError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustRecord {
    owner_id: OwnerId,
    device_id: DeviceId,
    state: TrustState,
    accepted_credential_epoch: u64,
    trust_revision: u64,
    last_transition_id: TransitionId,
}

impl TrustRecord {
    pub const fn trusted(
        owner_id: OwnerId,
        device_id: DeviceId,
        credential_epoch: u64,
        transition_id: TransitionId,
    ) -> Self {
        Self {
            owner_id,
            device_id,
            state: TrustState::Trusted,
            accepted_credential_epoch: credential_epoch,
            trust_revision: 0,
            last_transition_id: transition_id,
        }
    }

    pub fn advance_credential_epoch(&mut self, next_epoch: u64) -> Result<(), TrustError> {
        if next_epoch <= self.accepted_credential_epoch {
            return Err(TrustError::StaleCredentialEpoch);
        }
        let expected = self
            .accepted_credential_epoch
            .checked_add(1)
            .ok_or(TrustError::UnexpectedCredentialEpoch)?;
        if next_epoch != expected {
            return Err(TrustError::UnexpectedCredentialEpoch);
        }

        self.accepted_credential_epoch = next_epoch;
        Ok(())
    }

    fn revoke(&mut self, transition_id: TransitionId) -> Result<(), TrustError> {
        if self.state == TrustState::Revoked {
            return Err(TrustError::AlreadyRevoked);
        }

        self.state = TrustState::Revoked;
        self.trust_revision = self
            .trust_revision
            .checked_add(1)
            .ok_or(TrustError::UnexpectedCredentialEpoch)?;
        self.last_transition_id = transition_id;
        Ok(())
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub const fn state(&self) -> TrustState {
        self.state
    }

    pub const fn accepted_credential_epoch(&self) -> u64 {
        self.accepted_credential_epoch
    }

    pub const fn trust_revision(&self) -> u64 {
        self.trust_revision
    }

    pub const fn last_transition_id(&self) -> TransitionId {
        self.last_transition_id
    }
}
