use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crosslab_agent::{
    PresenceAgentError, PresencePhase, TrustedPresenceAgent, TrustedSessionRoute,
};
use crosslab_crypto::SigningProvider;
use crosslab_identity_store::ProductIdentityState;
use tokio::runtime::Runtime;

use crate::{
    MobileLifecycleState, MobileRuntimeSnapshot,
    network::socket_addr,
    product_identity::{ForeignSigningProvider, MobileProductIdentityError, MobileSigningProvider},
    session_discovery::{MobileTrustedSessionDiscoveryProfile, profile_for_instance},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobilePresencePhase {
    Discovering,
    Connecting,
    Online,
    Reconnecting,
    Paused,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobilePresenceSnapshot {
    pub phase: MobilePresencePhase,
    pub runtime: Option<MobileRuntimeSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobilePresenceDiscovery {
    pub profile: MobileTrustedSessionDiscoveryProfile,
    pub listen_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobilePresenceError {
    Identity,
    NoTrustedPeers,
    Random,
    Bind,
    Thread,
    InvalidRoute,
    QueueFull,
    Closed,
    StateUnavailable,
}

impl core::fmt::Display for MobilePresenceError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::Identity => "trusted presence identity is unavailable",
            Self::NoTrustedPeers => "trusted presence requires at least one trusted peer",
            Self::Random => "trusted presence discovery identity generation failed",
            Self::Bind => "trusted presence listener could not start",
            Self::Thread => "trusted presence agent could not start",
            Self::InvalidRoute => "trusted presence route is invalid",
            Self::QueueFull => "trusted presence command queue is full",
            Self::Closed => "trusted presence agent is closed",
            Self::StateUnavailable => "trusted presence state is unavailable",
        })
    }
}

impl std::error::Error for MobilePresenceError {}

#[derive(uniffi::Object)]
pub struct MobileTrustedPresenceAgent {
    agent: Mutex<Option<TrustedPresenceAgent>>,
    status: Mutex<tokio::sync::watch::Receiver<crosslab_agent::PresenceSnapshot>>,
    wait_runtime: Mutex<Runtime>,
}

impl core::fmt::Debug for MobileTrustedPresenceAgent {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("MobileTrustedPresenceAgent([REDACTED])")
    }
}

#[uniffi::export]
impl MobileTrustedPresenceAgent {
    #[uniffi::constructor]
    pub fn new(
        identity_payload: Vec<u8>,
        local_device_signer: Arc<dyn MobileSigningProvider>,
    ) -> Result<Self, MobilePresenceError> {
        let identity = ProductIdentityState::decode(&identity_payload)
            .map_err(|_| MobilePresenceError::Identity)?;
        let signer = ForeignSigningProvider::new(local_device_signer)?;
        let signer: Arc<dyn SigningProvider + Send + Sync> = Arc::new(signer);
        let agent = TrustedPresenceAgent::spawn(identity, signer)?;
        let status = agent.subscribe_status();
        let wait_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|_| MobilePresenceError::Thread)?;

        Ok(Self {
            agent: Mutex::new(Some(agent)),
            status: Mutex::new(status),
            wait_runtime: Mutex::new(wait_runtime),
        })
    }

    pub fn discovery(&self) -> Result<MobilePresenceDiscovery, MobilePresenceError> {
        self.with_agent(|agent| Ok(to_mobile_discovery(agent.discovery())))
    }

    pub fn rotate_discovery(&self) -> Result<MobilePresenceDiscovery, MobilePresenceError> {
        self.with_agent(|agent| Ok(to_mobile_discovery(agent.rotate_discovery()?)))
    }

    pub fn candidate_available(
        &self,
        instance: String,
        address: Vec<u8>,
        port: i32,
        scope_id: i32,
    ) -> Result<(), MobilePresenceError> {
        let address =
            socket_addr(address, port, scope_id).ok_or(MobilePresenceError::InvalidRoute)?;
        let route = TrustedSessionRoute::new(instance, address)?;
        self.with_agent(|agent| {
            agent.candidate_available(route)?;
            Ok(())
        })
    }

    pub fn candidate_lost(&self, instance: String) -> Result<(), MobilePresenceError> {
        self.with_agent(|agent| {
            agent.candidate_lost(instance)?;
            Ok(())
        })
    }

    pub fn network_lost(&self) -> Result<(), MobilePresenceError> {
        self.with_agent(|agent| {
            agent.network_lost()?;
            Ok(())
        })
    }

    pub fn network_available(&self) -> Result<(), MobilePresenceError> {
        self.with_agent(|agent| {
            agent.network_available()?;
            Ok(())
        })
    }

    pub fn disconnect(&self) -> Result<(), MobilePresenceError> {
        self.with_agent(|agent| {
            agent.disconnect()?;
            Ok(())
        })
    }

    pub fn reconnect(&self) -> Result<(), MobilePresenceError> {
        self.with_agent(|agent| {
            agent.reconnect()?;
            Ok(())
        })
    }

    pub fn snapshot(&self) -> Result<MobilePresenceSnapshot, MobilePresenceError> {
        let status = self
            .status
            .lock()
            .map_err(|_| MobilePresenceError::StateUnavailable)?;
        Ok(to_mobile_snapshot(&status.borrow()))
    }

    pub fn wait_event(
        &self,
        timeout_ms: u64,
    ) -> Result<Option<MobilePresenceSnapshot>, MobilePresenceError> {
        let mut status = self
            .status
            .lock()
            .map_err(|_| MobilePresenceError::StateUnavailable)?;
        let runtime = self
            .wait_runtime
            .lock()
            .map_err(|_| MobilePresenceError::StateUnavailable)?;
        let changed = runtime.block_on(async {
            tokio::time::timeout(Duration::from_millis(timeout_ms), status.changed()).await
        });

        match changed {
            Ok(Ok(())) => Ok(Some(to_mobile_snapshot(&status.borrow_and_update()))),
            Ok(Err(_)) => Err(MobilePresenceError::Closed),
            Err(_) => Ok(None),
        }
    }

    pub fn shutdown(&self) -> Result<(), MobilePresenceError> {
        let mut agent = self
            .agent
            .lock()
            .map_err(|_| MobilePresenceError::StateUnavailable)?;
        agent.take();
        Ok(())
    }
}

impl MobileTrustedPresenceAgent {
    fn with_agent<T>(
        &self,
        operation: impl FnOnce(&TrustedPresenceAgent) -> Result<T, MobilePresenceError>,
    ) -> Result<T, MobilePresenceError> {
        let agent = self
            .agent
            .lock()
            .map_err(|_| MobilePresenceError::StateUnavailable)?;
        operation(agent.as_ref().ok_or(MobilePresenceError::Closed)?)
    }
}

fn to_mobile_discovery(
    discovery: crosslab_agent::PresenceDiscoveryInfo,
) -> MobilePresenceDiscovery {
    MobilePresenceDiscovery {
        profile: profile_for_instance(discovery.instance().to_owned()),
        listen_port: discovery.listen_port(),
    }
}

fn to_mobile_snapshot(snapshot: &crosslab_agent::PresenceSnapshot) -> MobilePresenceSnapshot {
    MobilePresenceSnapshot {
        phase: match snapshot.phase() {
            PresencePhase::Discovering => MobilePresencePhase::Discovering,
            PresencePhase::Connecting => MobilePresencePhase::Connecting,
            PresencePhase::Online => MobilePresencePhase::Online,
            PresencePhase::Reconnecting => MobilePresencePhase::Reconnecting,
            PresencePhase::Paused => MobilePresencePhase::Paused,
            PresencePhase::Failed => MobilePresencePhase::Failed,
        },
        runtime: snapshot.runtime().map(|status| {
            MobileRuntimeSnapshot::from_runtime(status, MobileLifecycleState::Running, 0)
        }),
    }
}

impl From<PresenceAgentError> for MobilePresenceError {
    fn from(error: PresenceAgentError) -> Self {
        match error {
            PresenceAgentError::Identity => Self::Identity,
            PresenceAgentError::NoTrustedPeers => Self::NoTrustedPeers,
            PresenceAgentError::Random => Self::Random,
            PresenceAgentError::Bind => Self::Bind,
            PresenceAgentError::Thread => Self::Thread,
            PresenceAgentError::InvalidRoute => Self::InvalidRoute,
            PresenceAgentError::CommandQueueFull => Self::QueueFull,
            PresenceAgentError::Closed => Self::Closed,
        }
    }
}

impl From<MobileProductIdentityError> for MobilePresenceError {
    fn from(_: MobileProductIdentityError) -> Self {
        Self::Identity
    }
}
