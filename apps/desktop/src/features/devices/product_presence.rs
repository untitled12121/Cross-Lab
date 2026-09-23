use core::fmt;
use std::{sync::Arc, thread, time::Duration};

use crosslab_agent::{
    PresenceAgentError, PresenceSnapshot, TrustedPresenceAgent,
    TrustedSessionRoute as AgentTrustedSessionRoute,
};
use crosslab_identity_store::{ProductIdentityError, ProductIdentityState};
use tokio::sync::{mpsc, watch};

use crate::features::identity_store::{
    LinuxEd25519Signer, LinuxIdentityStore, LinuxIdentityStoreError, LinuxSigningSlot,
};

use super::{
    LinuxTrustedSessionDiscovery, LinuxTrustedSessionDiscoveryError,
    LinuxTrustedSessionDiscoveryEvent,
};

const DISCOVERY_RETRY: Duration = Duration::from_secs(2);
const CONTROL_CAPACITY: usize = 8;

pub struct DesktopProductPresenceController {
    agent: Arc<TrustedPresenceAgent>,
    status: watch::Receiver<PresenceSnapshot>,
    control_tx: mpsc::Sender<DiscoveryControl>,
}

impl DesktopProductPresenceController {
    pub async fn start() -> Result<Option<Self>, DesktopPresenceError> {
        let store = LinuxIdentityStore::from_environment()?;
        let Some(payload) = store.load_payload().await? else {
            return Ok(None);
        };

        let identity = ProductIdentityState::decode(&payload)?;
        if identity.trusted_peers().is_empty() {
            return Ok(None);
        }

        let signer = LinuxEd25519Signer::load_required(LinuxSigningSlot::LocalDevice).await?;
        let agent = Arc::new(TrustedPresenceAgent::spawn(identity, Arc::new(signer))?);
        let status = agent.subscribe_status();
        let discovery_agent = Arc::clone(&agent);
        let (control_tx, control_rx) = mpsc::channel(CONTROL_CAPACITY);

        thread::Builder::new()
            .name("crosslab-desktop-presence".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    let _ = discovery_agent.network_lost();
                    return;
                };
                runtime.block_on(run_discovery(discovery_agent, control_rx));
            })
            .map_err(|_| DesktopPresenceError::Thread)?;

        Ok(Some(Self {
            agent,
            status,
            control_tx,
        }))
    }

    pub fn subscribe_status(&self) -> watch::Receiver<PresenceSnapshot> {
        self.status.clone()
    }

    pub fn disconnect(&self) -> Result<(), DesktopPresenceError> {
        self.agent.disconnect()?;
        self.send_control(DiscoveryControl::Pause)
    }

    pub fn reconnect(&self) -> Result<(), DesktopPresenceError> {
        self.agent.rotate_discovery()?;
        self.agent.reconnect()?;
        self.send_control(DiscoveryControl::Resume)
    }

    fn send_control(&self, control: DiscoveryControl) -> Result<(), DesktopPresenceError> {
        self.control_tx
            .try_send(control)
            .map_err(|_| DesktopPresenceError::Control)
    }
}

impl Drop for DesktopProductPresenceController {
    fn drop(&mut self) {
        let _ = self.control_tx.try_send(DiscoveryControl::Stop);
    }
}

#[derive(Debug)]
pub enum DesktopPresenceError {
    IdentityStore(LinuxIdentityStoreError),
    ProductIdentity(ProductIdentityError),
    Agent(PresenceAgentError),
    Discovery(LinuxTrustedSessionDiscoveryError),
    Control,
    Thread,
}

impl fmt::Display for DesktopPresenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityStore(error) => fmt::Display::fmt(error, formatter),
            Self::ProductIdentity(error) => fmt::Display::fmt(error, formatter),
            Self::Agent(error) => fmt::Display::fmt(error, formatter),
            Self::Discovery(error) => fmt::Display::fmt(error, formatter),
            Self::Control => formatter.write_str("desktop presence control queue is unavailable"),
            Self::Thread => formatter.write_str("desktop presence supervisor could not start"),
        }
    }
}

impl std::error::Error for DesktopPresenceError {}

impl From<LinuxIdentityStoreError> for DesktopPresenceError {
    fn from(error: LinuxIdentityStoreError) -> Self {
        Self::IdentityStore(error)
    }
}

impl From<ProductIdentityError> for DesktopPresenceError {
    fn from(error: ProductIdentityError) -> Self {
        Self::ProductIdentity(error)
    }
}

impl From<PresenceAgentError> for DesktopPresenceError {
    fn from(error: PresenceAgentError) -> Self {
        Self::Agent(error)
    }
}

impl From<LinuxTrustedSessionDiscoveryError> for DesktopPresenceError {
    fn from(error: LinuxTrustedSessionDiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscoveryControl {
    Pause,
    Resume,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscoveryLoopExit {
    Paused,
    Failed,
    Stop,
}

async fn run_discovery(
    agent: Arc<TrustedPresenceAgent>,
    mut control_rx: mpsc::Receiver<DiscoveryControl>,
) {
    let mut enabled = true;

    loop {
        if !enabled {
            match control_rx.recv().await {
                Some(DiscoveryControl::Resume) => enabled = true,
                Some(DiscoveryControl::Pause) => {}
                Some(DiscoveryControl::Stop) | None => return,
            }
            continue;
        }

        let discovery_info = agent.discovery();
        let discovery = tokio::select! {
            control = control_rx.recv() => {
                match control {
                    Some(DiscoveryControl::Pause) => {
                        enabled = false;
                        continue;
                    }
                    Some(DiscoveryControl::Resume) => continue,
                    Some(DiscoveryControl::Stop) | None => return,
                }
            }
            result = LinuxTrustedSessionDiscovery::start(
                discovery_info.instance().to_owned(),
                discovery_info.listen_port(),
            ) => {
                match result {
                    Ok(discovery) => discovery,
                    Err(_) => {
                        let _ = agent.network_lost();
                        let _ = agent.rotate_discovery();
                        match wait_retry_or_control(&mut control_rx).await {
                            RetryOutcome::Retry => {}
                            RetryOutcome::Paused => enabled = false,
                            RetryOutcome::Stop => return,
                        }
                        continue;
                    }
                }
            }
        };

        let _ = agent.network_available();
        let mut discovery = Some(discovery);
        let exit = loop {
            let active = discovery
                .as_mut()
                .expect("discovery exists until the loop exits");
            tokio::select! {
                control = control_rx.recv() => {
                    match control {
                        Some(DiscoveryControl::Pause) => {
                            break DiscoveryLoopExit::Paused;
                        }
                        Some(DiscoveryControl::Resume) => {}
                        Some(DiscoveryControl::Stop) | None => {
                            break DiscoveryLoopExit::Stop;
                        }
                    }
                }
                event = active.next_event() => {
                    match event {
                        Some(LinuxTrustedSessionDiscoveryEvent::Candidate(route)) => {
                            if let Ok(route) =
                                AgentTrustedSessionRoute::new(
                                    route.instance().to_owned(),
                                    route.address(),
                                )
                            {
                                let _ = agent.candidate_available(route);
                            }
                        }
                        Some(LinuxTrustedSessionDiscoveryEvent::Lost(instance)) => {
                            let _ = agent.candidate_lost(instance);
                        }
                        Some(LinuxTrustedSessionDiscoveryEvent::Failed) | None => {
                            break DiscoveryLoopExit::Failed;
                        }
                    }
                }
            }
        };

        if let Some(discovery) = discovery.take() {
            let _ = discovery.stop().await;
        }

        match exit {
            DiscoveryLoopExit::Stop => return,
            DiscoveryLoopExit::Paused => {
                enabled = false;
            }
            DiscoveryLoopExit::Failed => {
                let _ = agent.network_lost();
                let _ = agent.rotate_discovery();
                match wait_retry_or_control(&mut control_rx).await {
                    RetryOutcome::Retry => {}
                    RetryOutcome::Paused => enabled = false,
                    RetryOutcome::Stop => return,
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetryOutcome {
    Retry,
    Paused,
    Stop,
}

async fn wait_retry_or_control(
    control_rx: &mut mpsc::Receiver<DiscoveryControl>,
) -> RetryOutcome {
    tokio::select! {
        control = control_rx.recv() => {
            match control {
                Some(DiscoveryControl::Pause) => RetryOutcome::Paused,
                Some(DiscoveryControl::Resume) => RetryOutcome::Retry,
                Some(DiscoveryControl::Stop) | None => RetryOutcome::Stop,
            }
        }
        _ = tokio::time::sleep(DISCOVERY_RETRY) => RetryOutcome::Retry,
    }
}
