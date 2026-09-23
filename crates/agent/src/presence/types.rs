use core::fmt;
use std::net::SocketAddr;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceAgentError {
    Identity,
    NoTrustedPeers,
    Random,
    Bind,
    Thread,
    InvalidRoute,
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
            Self::CommandQueueFull => "trusted presence command queue is full",
            Self::Closed => "trusted presence agent is closed",
        })
    }
}

impl std::error::Error for PresenceAgentError {}
