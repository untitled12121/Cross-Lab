use core::fmt;

use crosslab_identity::{DeviceId, OwnerId};

use super::{PairingId, PairingSecret};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingInvitationState {
    Pending,
    Consumed,
    Cancelled,
    Expired,
}

#[derive(Debug)]
pub struct PairingInvitation {
    pairing_id: PairingId,
    secret: PairingSecret,
    owner_id: OwnerId,
    inviter_device_id: DeviceId,
    state: PairingInvitationState,
}

impl PairingInvitation {
    pub fn from_parts(
        pairing_id: PairingId,
        secret: PairingSecret,
        owner_id: OwnerId,
        inviter_device_id: DeviceId,
    ) -> Self {
        Self {
            pairing_id,
            secret,
            owner_id,
            inviter_device_id,
            state: PairingInvitationState::Pending,
        }
    }

    pub const fn pairing_id(&self) -> PairingId {
        self.pairing_id
    }

    pub const fn secret(&self) -> &PairingSecret {
        &self.secret
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn inviter_device_id(&self) -> DeviceId {
        self.inviter_device_id
    }

    pub const fn state(&self) -> PairingInvitationState {
        self.state
    }

    pub fn consume(&mut self) -> Result<(), PairingInvitationError> {
        self.transition(PairingInvitationState::Consumed)
    }

    pub fn cancel(&mut self) -> Result<(), PairingInvitationError> {
        self.transition(PairingInvitationState::Cancelled)
    }

    pub fn expire(&mut self) -> Result<(), PairingInvitationError> {
        self.transition(PairingInvitationState::Expired)
    }

    fn transition(
        &mut self,
        next: PairingInvitationState,
    ) -> Result<(), PairingInvitationError> {
        if self.state != PairingInvitationState::Pending {
            return Err(PairingInvitationError::NotPending);
        }

        self.state = next;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingInvitationError {
    NotPending,
}

impl fmt::Display for PairingInvitationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPending => formatter.write_str("pairing invitation is no longer pending"),
        }
    }
}

impl std::error::Error for PairingInvitationError {}
