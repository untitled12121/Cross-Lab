use core::{fmt, num::NonZeroU32};

use crosslab_crypto::random_bytes;
use crosslab_identity::DeviceId;

use crate::{
    AuthorizationGrant, CapabilityId, CapabilityVersion, Constraint, OperationName, SessionId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId([u8; 32]);

impl OperationId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsePolicy {
    SingleAction,
    SingleStream,
    MultiStream { max_streams: NonZeroU32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    Active,
    Cancelled,
    Expired,
    Revoked,
    Consumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationError {
    RandomnessUnavailable,
    InvalidLifetime,
    BindingMismatch,
    TrustRevisionChanged,
    PolicyRevisionChanged,
    StreamUseNotAllowed,
    StreamBudgetExhausted,
    Inactive(OperationState),
}

impl fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RandomnessUnavailable => "secure randomness is unavailable",
            Self::InvalidLifetime => "authorized operation lifetime is invalid",
            Self::BindingMismatch => "authorized operation binding does not match",
            Self::TrustRevisionChanged => "trust revision changed",
            Self::PolicyRevisionChanged => "policy revision changed",
            Self::StreamUseNotAllowed => "authorized operation does not permit data streams",
            Self::StreamBudgetExhausted => "authorized operation stream budget is exhausted",
            Self::Inactive(_) => "authorized operation is not active",
        })
    }
}

impl std::error::Error for OperationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationUseContext {
    source_device_id: DeviceId,
    destination_device_id: DeviceId,
    session_id: SessionId,
    capability_id: CapabilityId,
    capability_version: CapabilityVersion,
    operation: OperationName,
}

impl OperationUseContext {
    pub fn new(
        source_device_id: DeviceId,
        destination_device_id: DeviceId,
        session_id: SessionId,
        capability_id: CapabilityId,
        capability_version: CapabilityVersion,
        operation: OperationName,
    ) -> Self {
        Self {
            source_device_id,
            destination_device_id,
            session_id,
            capability_id,
            capability_version,
            operation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedOperation {
    id: OperationId,
    source_device_id: DeviceId,
    destination_device_id: DeviceId,
    session_id: SessionId,
    capability_id: CapabilityId,
    capability_version: CapabilityVersion,
    operation: OperationName,
    trust_revision: u64,
    policy_revision: u64,
    constraints_snapshot: Vec<Constraint>,
    created_at: u64,
    expires_at: u64,
    use_policy: UsePolicy,
    reserved_stream_uses: u32,
    state: OperationState,
}

impl AuthorizedOperation {
    pub fn issue(
        grant: AuthorizationGrant,
        created_at: u64,
        expires_at: u64,
        use_policy: UsePolicy,
    ) -> Result<Self, OperationError> {
        if created_at >= expires_at {
            return Err(OperationError::InvalidLifetime);
        }

        let id =
            OperationId(random_bytes::<32>().map_err(|_| OperationError::RandomnessUnavailable)?);
        Ok(Self {
            id,
            source_device_id: grant.source_device_id,
            destination_device_id: grant.destination_device_id,
            session_id: grant.session_id,
            capability_id: grant.capability_id,
            capability_version: grant.capability_version,
            operation: grant.operation,
            trust_revision: grant.trust_revision,
            policy_revision: grant.policy_revision,
            constraints_snapshot: grant.constraints_snapshot,
            created_at,
            expires_at,
            use_policy,
            reserved_stream_uses: 0,
            state: OperationState::Active,
        })
    }

    pub fn validate(
        &mut self,
        context: &OperationUseContext,
        now: u64,
        current_trust_revision: u64,
        current_policy_revision: u64,
    ) -> Result<(), OperationError> {
        if self.state != OperationState::Active {
            return Err(OperationError::Inactive(self.state));
        }
        if now >= self.expires_at {
            self.state = OperationState::Expired;
            return Err(OperationError::Inactive(OperationState::Expired));
        }
        if current_trust_revision != self.trust_revision {
            self.state = OperationState::Revoked;
            return Err(OperationError::TrustRevisionChanged);
        }
        if current_policy_revision != self.policy_revision {
            self.state = OperationState::Revoked;
            return Err(OperationError::PolicyRevisionChanged);
        }
        if self.source_device_id != context.source_device_id
            || self.destination_device_id != context.destination_device_id
            || self.session_id != context.session_id
            || self.capability_id != context.capability_id
            || self.capability_version != context.capability_version
            || self.operation != context.operation
        {
            return Err(OperationError::BindingMismatch);
        }
        Ok(())
    }

    pub fn reserve_stream_use(
        &mut self,
        context: &OperationUseContext,
        now: u64,
        current_trust_revision: u64,
        current_policy_revision: u64,
    ) -> Result<(), OperationError> {
        self.validate(
            context,
            now,
            current_trust_revision,
            current_policy_revision,
        )?;

        let Some(budget) = stream_budget(self.use_policy) else {
            return Err(OperationError::StreamUseNotAllowed);
        };
        if self.reserved_stream_uses >= budget {
            return Err(OperationError::StreamBudgetExhausted);
        }

        self.reserved_stream_uses += 1;
        Ok(())
    }

    pub fn cancel(&mut self) {
        if self.state == OperationState::Active {
            self.state = OperationState::Cancelled;
        }
    }

    pub fn revoke(&mut self) {
        if self.state == OperationState::Active {
            self.state = OperationState::Revoked;
        }
    }

    pub fn consume(&mut self) {
        if self.state == OperationState::Active {
            self.state = OperationState::Consumed;
        }
    }

    pub const fn id(&self) -> OperationId {
        self.id
    }

    pub const fn state(&self) -> OperationState {
        self.state
    }

    pub fn constraints_snapshot(&self) -> &[Constraint] {
        &self.constraints_snapshot
    }

    pub const fn created_at(&self) -> u64 {
        self.created_at
    }

    pub const fn expires_at(&self) -> u64 {
        self.expires_at
    }

    pub const fn use_policy(&self) -> UsePolicy {
        self.use_policy
    }

    pub const fn reserved_stream_uses(&self) -> u32 {
        self.reserved_stream_uses
    }

    pub const fn stream_budget_exhausted(&self) -> bool {
        match stream_budget(self.use_policy) {
            Some(budget) => self.reserved_stream_uses >= budget,
            None => true,
        }
    }
}

const fn stream_budget(use_policy: UsePolicy) -> Option<u32> {
    match use_policy {
        UsePolicy::SingleAction => None,
        UsePolicy::SingleStream => Some(1),
        UsePolicy::MultiStream { max_streams } => Some(max_streams.get()),
    }
}
