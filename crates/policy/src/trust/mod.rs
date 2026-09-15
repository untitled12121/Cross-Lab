use core::fmt;

use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError,
    OwnerAuthorityState, OwnerId, OwnerRootRecord,
};

use crate::id_generation::{PolicyIdGenerationError, random_policy_id};

mod pairing;
mod transition;

pub use pairing::{PairingTrustTransition, PairingTrustTransitionError};
pub use transition::{TrustTransition, TrustTransitionError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionId([u8; 32]);

impl TransitionId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn generate() -> Result<Self, PolicyIdGenerationError> {
        random_policy_id().map(Self)
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
pub enum CredentialRotationError {
    Identity(IdentityError),
    WrongOwner,
    WrongDevice,
    NotTrusted,
    StaleCredentialEpoch,
    UnexpectedCredentialEpoch,
    RevisionOverflow,
}

impl fmt::Display for CredentialRotationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(error) => fmt::Display::fmt(error, formatter),
            Self::WrongOwner => formatter.write_str("credential belongs to a different owner"),
            Self::WrongDevice => formatter.write_str("credential belongs to a different device"),
            Self::NotTrusted => formatter.write_str("device trust is not active"),
            Self::StaleCredentialEpoch => formatter.write_str("credential epoch is stale"),
            Self::UnexpectedCredentialEpoch => {
                formatter.write_str("credential epoch transition is invalid")
            }
            Self::RevisionOverflow => formatter.write_str("trust revision is exhausted"),
        }
    }
}

impl std::error::Error for CredentialRotationError {}

impl From<IdentityError> for CredentialRotationError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}

/// Locally authoritative trusted state must be established through verified pairing evidence.
///
/// Ordinary callers cannot mint trusted state directly:
///
/// ```compile_fail
/// use crosslab_identity::{DeviceId, OwnerId};
/// use crosslab_policy::{TransitionId, TrustRecord};
///
/// let _ = TrustRecord::trusted(
///     OwnerId::from_bytes([0x11; 32]),
///     DeviceId::from_bytes([0x22; 32]),
///     0,
///     TransitionId::from_bytes([0x33; 32]),
/// );
/// ```
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
    pub(super) const fn established_from_pairing(
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

    fn accept_successor_credential_with_authority_parts(
        &mut self,
        successor: &DeviceCredential,
        root: &OwnerRootRecord,
        issuer: &AuthorityDelegation,
        minimum_delegation_epoch: u64,
        transition_id: TransitionId,
    ) -> Result<(), CredentialRotationError> {
        if self.state != TrustState::Trusted {
            return Err(CredentialRotationError::NotTrusted);
        }
        if successor.owner_id() != self.owner_id {
            return Err(CredentialRotationError::WrongOwner);
        }
        if successor.device_id() != self.device_id {
            return Err(CredentialRotationError::WrongDevice);
        }
        if successor.credential_epoch() <= self.accepted_credential_epoch {
            return Err(CredentialRotationError::StaleCredentialEpoch);
        }
        let expected_epoch = self
            .accepted_credential_epoch
            .checked_add(1)
            .ok_or(CredentialRotationError::UnexpectedCredentialEpoch)?;
        if successor.credential_epoch() != expected_epoch {
            return Err(CredentialRotationError::UnexpectedCredentialEpoch);
        }

        successor.verify(root, issuer, expected_epoch, minimum_delegation_epoch)?;
        let next_revision = self
            .trust_revision
            .checked_add(1)
            .ok_or(CredentialRotationError::RevisionOverflow)?;

        self.accepted_credential_epoch = expected_epoch;
        self.trust_revision = next_revision;
        self.last_transition_id = transition_id;
        Ok(())
    }

    pub fn accept_successor_credential(
        &mut self,
        successor: &DeviceCredential,
        authority: &OwnerAuthorityState,
        transition_id: TransitionId,
    ) -> Result<(), CredentialRotationError> {
        let issuer = authority.current_delegation(AuthorityRole::DeviceSigning)?;
        self.accept_successor_credential_with_authority_parts(
            successor,
            authority.root(),
            issuer,
            issuer.delegation_epoch(),
            transition_id,
        )
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
