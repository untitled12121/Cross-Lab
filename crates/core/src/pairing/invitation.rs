use core::fmt;

use crosslab_identity::{DeviceId, OwnerId};

use super::{PairingId, PairingSecret};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PairingInstant(u64);

impl PairingInstant {
    pub const fn from_ticks(ticks: u64) -> Self {
        Self(ticks)
    }

    pub const fn ticks(self) -> u64 {
        self.0
    }
}

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
    created_at: PairingInstant,
    deadline: PairingInstant,
    state: PairingInvitationState,
}

impl PairingInvitation {
    pub fn from_parts(
        pairing_id: PairingId,
        secret: PairingSecret,
        owner_id: OwnerId,
        inviter_device_id: DeviceId,
        created_at: PairingInstant,
        deadline: PairingInstant,
    ) -> Result<Self, PairingInvitationError> {
        if deadline <= created_at {
            return Err(PairingInvitationError::InvalidDeadline);
        }

        Ok(Self {
            pairing_id,
            secret,
            owner_id,
            inviter_device_id,
            created_at,
            deadline,
            state: PairingInvitationState::Pending,
        })
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

    pub const fn created_at(&self) -> PairingInstant {
        self.created_at
    }

    pub const fn deadline(&self) -> PairingInstant {
        self.deadline
    }

    pub const fn state(&self) -> PairingInvitationState {
        self.state
    }

    pub fn ensure_pending_at(&mut self, now: PairingInstant) -> Result<(), PairingInvitationError> {
        if self.state != PairingInvitationState::Pending {
            return Err(PairingInvitationError::NotPending);
        }
        if now >= self.deadline {
            self.state = PairingInvitationState::Expired;
            return Err(PairingInvitationError::Expired);
        }

        Ok(())
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

    fn transition(&mut self, next: PairingInvitationState) -> Result<(), PairingInvitationError> {
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
    Expired,
    InvalidDeadline,
}

impl fmt::Display for PairingInvitationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotPending => "pairing invitation is no longer pending",
            Self::Expired => "pairing invitation has expired",
            Self::InvalidDeadline => "pairing invitation deadline must be after creation",
        })
    }
}

impl std::error::Error for PairingInvitationError {}
