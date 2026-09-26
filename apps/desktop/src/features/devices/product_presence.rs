use core::fmt;
use std::{sync::Arc, thread, time::Duration};

use crosslab_agent::{
    ClipboardAvailability, ClipboardOperationError, ClipboardPlatformError, ClipboardRequest,
    PermissionSnapshot, PresenceAgentError, PresenceSnapshot, TrustedPresenceAgent,
    TrustedSessionRoute as AgentTrustedSessionRoute,
};
use crosslab_identity_store::{ProductIdentityError, ProductIdentityState};
use crosslab_policy::{CapabilityId, OperationName, PolicyError, RuleEffect};
use tokio::sync::{mpsc, watch};

use crate::features::{
    identity_store::{
        LinuxEd25519Signer, LinuxIdentityStore, LinuxIdentityStoreError, LinuxSigningSlot,
    },
    policy_store::{LinuxPolicyStore, LinuxPolicyStoreError},
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

        let policy = LinuxPolicyStore::from_environment()?.load().await?;
        let signer = LinuxEd25519Signer::load_required(LinuxSigningSlot::LocalDevice).await?;
        let agent = Arc::new(TrustedPresenceAgent::spawn_with_policy_and_clipboard(
            identity,
            Arc::new(signer),
            policy,
            ClipboardAvailability::new(true, true),
        )?);
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

    pub fn subscribe_permissions(&self) -> watch::Receiver<PermissionSnapshot> {
        self.agent.subscribe_permissions()
    }

    pub fn take_clipboard_requests(
        &self,
    ) -> Result<mpsc::Receiver<ClipboardRequest>, ClipboardOperationError> {
        self.agent.take_clipboard_requests()
    }

    pub async fn send_clipboard_text(
        &self,
        text: String,
    ) -> Result<(), ClipboardOperationError> {
        self.agent.send_clipboard_text(text).await
    }

    pub async fn fetch_clipboard_text(&self) -> Result<String, ClipboardOperationError> {
        self.agent.fetch_clipboard_text().await
    }

    pub async fn complete_clipboard_read(
        &self,
        request_id: crosslab_protocol::RequestId,
        result: Result<String, ClipboardPlatformError>,
    ) -> Result<(), ClipboardOperationError> {
        self.agent.complete_clipboard_read(request_id, result).await
    }

    pub async fn complete_clipboard_write(
        &self,
        request_id: crosslab_protocol::RequestId,
        result: Result<(), ClipboardPlatformError>,
    ) -> Result<(), ClipboardOperationError> {
        self.agent.complete_clipboard_write(request_id, result).await
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

    pub async fn set_permission(
        &self,
        capability_id: CapabilityId,
        operation: OperationName,
        effect: RuleEffect,
    ) -> Result<bool, DesktopPresenceError> {
        let peer_device_id = self
            .status
            .borrow()
            .runtime()
            .and_then(|runtime| runtime.peer_device_id())
            .ok_or(DesktopPresenceError::PeerUnavailable)?;
        let store = LinuxPolicyStore::from_environment()?;
        let mut policy = match store.load().await {
            Ok(policy) => policy,
            Err(error) => {
                let _ = self.agent.fail_closed_policy().await;
                return Err(error.into());
            }
        };
        let expected_revision = policy.revision();
        if expected_revision != self.agent.permission_snapshot().policy_revision() {
            let _ = self.agent.fail_closed_policy().await;
            return Err(DesktopPresenceError::PolicyOutOfSync);
        }
        if !policy.set_rule_effect(peer_device_id, capability_id, operation, effect)? {
            return Ok(false);
        }

        match store.commit(expected_revision, &policy).await {
            Ok(revision) if revision == policy.revision() => {}
            Ok(_) => {
                let _ = self.agent.fail_closed_policy().await;
                return Err(DesktopPresenceError::PolicyOutOfSync);
            }
            Err(error) => {
                match store.load().await {
                    Ok(durable) if durable.revision() == expected_revision => {
                        return Err(error.into());
                    }
                    Ok(durable) if durable == policy => {
                        self.agent.replace_policy_and_wait(durable).await?;
                        return Ok(true);
                    }
                    Ok(_) | Err(_) => {
                        let _ = self.agent.fail_closed_policy().await;
                    }
                }
                return Err(error.into());
            }
        }

        self.agent.replace_policy_and_wait(policy).await?;
        Ok(true)
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
    PolicyStore(LinuxPolicyStoreError),
    Policy(PolicyError),
    ProductIdentity(ProductIdentityError),
    Agent(PresenceAgentError),
    Discovery(LinuxTrustedSessionDiscoveryError),
    PeerUnavailable,
    PolicyOutOfSync,
    Control,
    Thread,
}

impl fmt::Display for DesktopPresenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityStore(error) => fmt::Display::fmt(error, formatter),
            Self::PolicyStore(error) => fmt::Display::fmt(error, formatter),
            Self::Policy(_) => formatter.write_str("desktop policy mutation failed"),
            Self::ProductIdentity(error) => fmt::Display::fmt(error, formatter),
            Self::Agent(error) => fmt::Display::fmt(error, formatter),
            Self::Discovery(error) => fmt::Display::fmt(error, formatter),
            Self::PeerUnavailable => formatter.write_str("desktop permission peer is unavailable"),
            Self::PolicyOutOfSync => {
                formatter.write_str("desktop active policy does not match durable policy")
            }
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

impl From<LinuxPolicyStoreError> for DesktopPresenceError {
    fn from(error: LinuxPolicyStoreError) -> Self {
        Self::PolicyStore(error)
    }
}

impl From<PolicyError> for DesktopPresenceError {
    fn from(error: PolicyError) -> Self {
        Self::Policy(error)
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

async fn wait_retry_or_control(control_rx: &mut mpsc::Receiver<DiscoveryControl>) -> RetryOutcome {
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
