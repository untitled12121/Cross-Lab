use core::fmt;
use std::net::SocketAddr;

use crosslab_identity::DeviceId;
use crosslab_policy::{CapabilityId, OperationName, PolicyState, RuleEffect};
use crosslab_runtime::RuntimeStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceDiscoveryInfo {
    instance: String,
    listen_port: u16,
}

impl PresenceDiscoveryInfo {
    pub(crate) fn new(instance: String, listen_port: u16) -> Self {
        Self {
            instance,
            listen_port,
        }
    }

    pub fn instance(&self) -> &str {
        &self.instance
    }

    pub const fn listen_port(&self) -> u16 {
        self.listen_port
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedSessionRoute {
    instance: String,
    address: SocketAddr,
}

impl TrustedSessionRoute {
    pub fn new(instance: String, address: SocketAddr) -> Result<Self, PresenceAgentError> {
        if !crosslab_core::is_session_dns_sd_instance(&instance) || address.port() == 0 {
            return Err(PresenceAgentError::InvalidRoute);
        }
        Ok(Self { instance, address })
    }

    pub fn instance(&self) -> &str {
        &self.instance
    }

    pub const fn address(&self) -> SocketAddr {
        self.address
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresencePhase {
    Discovering,
    Connecting,
    Online,
    Reconnecting,
    Paused,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSnapshot {
    phase: PresencePhase,
    runtime: Option<RuntimeStatus>,
}

impl PresenceSnapshot {
    pub(crate) fn new(phase: PresencePhase, runtime: Option<RuntimeStatus>) -> Self {
        Self { phase, runtime }
    }

    pub const fn phase(&self) -> PresencePhase {
        self.phase
    }

    pub const fn runtime(&self) -> Option<&RuntimeStatus> {
        self.runtime.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRule {
    source_device_id: DeviceId,
    capability_id: CapabilityId,
    operation: OperationName,
    effect: RuleEffect,
}

impl PermissionRule {
    pub const fn source_device_id(&self) -> DeviceId {
        self.source_device_id
    }

    pub const fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
    }

    pub const fn operation(&self) -> &OperationName {
        &self.operation
    }

    pub const fn effect(&self) -> RuleEffect {
        self.effect
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionSnapshot {
    policy_revision: u64,
    rules: Vec<PermissionRule>,
}

impl PermissionSnapshot {
    pub(crate) fn from_policy(policy: &PolicyState) -> Self {
        Self {
            policy_revision: policy.revision(),
            rules: policy
                .rules()
                .map(|rule| PermissionRule {
                    source_device_id: rule.source_device_id(),
                    capability_id: rule.capability_id().clone(),
                    operation: rule.operation().clone(),
                    effect: rule.effect(),
                })
                .collect(),
        }
    }

    pub const fn policy_revision(&self) -> u64 {
        self.policy_revision
    }

    pub fn rules(&self) -> &[PermissionRule] {
        &self.rules
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceAgentError {
    Identity,
    NoTrustedPeers,
    Random,
    Bind,
    Thread,
    InvalidRoute,
    StalePolicy,
    PolicyRevisionExhausted,
    CommandQueueFull,
    Closed,
}

impl fmt::Display for PresenceAgentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Identity => "trusted presence identity is unavailable",
            Self::NoTrustedPeers => "trusted presence requires at least one trusted peer",
            Self::Random => "trusted presence discovery identity generation failed",
            Self::Bind => "trusted presence listener could not start",
            Self::Thread => "trusted presence agent could not start",
            Self::InvalidRoute => "trusted presence route is invalid",
            Self::StalePolicy => "trusted presence policy revision is stale",
            Self::PolicyRevisionExhausted => "trusted presence policy revision is exhausted",
            Self::CommandQueueFull => "trusted presence command queue is full",
            Self::Closed => "trusted presence agent is closed",
        })
    }
}

impl std::error::Error for PresenceAgentError {}
