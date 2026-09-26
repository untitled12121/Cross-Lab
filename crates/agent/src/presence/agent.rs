use std::sync::mpsc as std_mpsc;
use std::{
    net::{Ipv6Addr, SocketAddr},
    sync::{Arc, Mutex, RwLock},
    thread,
};

use crosslab_core::{SESSION_DNS_SD_INSTANCE_NONCE_LEN, session_dns_sd_instance};
use crosslab_crypto::{SigningProvider, random_bytes};
use crosslab_identity_store::ProductIdentityState;
use crosslab_policy::PolicyState;
use crosslab_transport_quic::{QuicTransportConfig, TrustedSessionQuicServer};
use tokio::sync::{mpsc, oneshot, watch};

use super::{
    runner::{AgentChannels, AgentCommand, AgentSecurity, run_agent},
    types::{
        PermissionSnapshot, PresenceAgentError, PresenceDiscoveryInfo, PresencePhase,
        PresenceSnapshot, TrustedSessionRoute,
    },
};
use crate::clipboard::{
    ClipboardAvailability, ClipboardOperationError, ClipboardPlatformError, ClipboardRequest,
};

const COMMAND_CAPACITY: usize = 64;
const CLIPBOARD_REQUEST_CAPACITY: usize = 8;

pub struct TrustedPresenceAgent {
    discovery_instance: Arc<RwLock<String>>,
    listen_port: u16,
    command_tx: mpsc::Sender<AgentCommand>,
    policy_tx: watch::Sender<PolicyState>,
    status: watch::Receiver<PresenceSnapshot>,
    permissions: watch::Receiver<PermissionSnapshot>,
    clipboard_requests: Mutex<Option<mpsc::Receiver<ClipboardRequest>>>,
}

impl TrustedPresenceAgent {
    pub fn spawn(
        identity: ProductIdentityState,
        signer: Arc<dyn SigningProvider + Send + Sync>,
    ) -> Result<Self, PresenceAgentError> {
        Self::spawn_with_policy(identity, signer, PolicyState::new())
    }

    pub fn spawn_with_policy(
        identity: ProductIdentityState,
        signer: Arc<dyn SigningProvider + Send + Sync>,
        policy: PolicyState,
    ) -> Result<Self, PresenceAgentError> {
        Self::spawn_with_policy_and_clipboard(
            identity,
            signer,
            policy,
            ClipboardAvailability::default(),
        )
    }

    pub fn spawn_with_policy_and_clipboard(
        identity: ProductIdentityState,
        signer: Arc<dyn SigningProvider + Send + Sync>,
        policy: PolicyState,
        clipboard_availability: ClipboardAvailability,
    ) -> Result<Self, PresenceAgentError> {
        identity
            .validate_local_device_provider(signer.as_ref())
            .map_err(|_| PresenceAgentError::Identity)?;
        let authority = identity
            .authority_state()
            .map_err(|_| PresenceAgentError::Identity)?;
        let trusts = identity
            .trusted_peer_records()
            .map_err(|_| PresenceAgentError::Identity)?;
        if trusts.is_empty() {
            return Err(PresenceAgentError::NoTrustedPeers);
        }

        let nonce = random_bytes::<SESSION_DNS_SD_INSTANCE_NONCE_LEN>()
            .map_err(|_| PresenceAgentError::Random)?;
        let instance = session_dns_sd_instance(nonce);
        let discovery_instance = Arc::new(RwLock::new(instance.clone()));
        let security = Arc::new(AgentSecurity {
            authority,
            local_credential: identity.local_credential(),
            signer,
            trusts,
        });
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
        let (policy_tx, policy_rx) = watch::channel(policy.clone());
        let (status_tx, status) =
            watch::channel(PresenceSnapshot::new(PresencePhase::Discovering, None));
        let (permissions_tx, permissions) =
            watch::channel(PermissionSnapshot::from_policy(&policy));
        let (clipboard_requests_tx, clipboard_requests) = mpsc::channel(CLIPBOARD_REQUEST_CAPACITY);
        let (startup_tx, startup_rx) = std_mpsc::sync_channel(1);
        let runner_discovery_instance = Arc::clone(&discovery_instance);
        thread::Builder::new()
            .name("crosslab-presence-agent".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    let _ = startup_tx.send(Err(PresenceAgentError::Thread));
                    status_tx.send_replace(PresenceSnapshot::new(PresencePhase::Failed, None));
                    return;
                };
                let local = tokio::task::LocalSet::new();
                local.block_on(&runtime, async move {
                    let server = match TrustedSessionQuicServer::bind(
                        SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
                        QuicTransportConfig::default(),
                    ) {
                        Ok(server) => server,
                        Err(_) => {
                            let _ = startup_tx.send(Err(PresenceAgentError::Bind));
                            status_tx
                                .send_replace(PresenceSnapshot::new(PresencePhase::Failed, None));
                            return;
                        }
                    };
                    let listen_port = match server.local_addr() {
                        Ok(address) => address.port(),
                        Err(_) => {
                            let _ = startup_tx.send(Err(PresenceAgentError::Bind));
                            status_tx
                                .send_replace(PresenceSnapshot::new(PresencePhase::Failed, None));
                            return;
                        }
                    };
                    if startup_tx.send(Ok(listen_port)).is_err() {
                        server.close();
                        return;
                    }
                    run_agent(
                        server,
                        security,
                        runner_discovery_instance,
                        policy,
                        AgentChannels::new(
                            policy_rx,
                            command_rx,
                            status_tx,
                            permissions_tx,
                            clipboard_requests_tx,
                            clipboard_availability,
                        ),
                    )
                    .await;
                });
            })
            .map_err(|_| PresenceAgentError::Thread)?;
        let listen_port = startup_rx
            .recv()
            .map_err(|_| PresenceAgentError::Thread)??;

        Ok(Self {
            discovery_instance,
            listen_port,
            command_tx,
            policy_tx,
            status,
            permissions,
            clipboard_requests: Mutex::new(Some(clipboard_requests)),
        })
    }

    pub fn discovery(&self) -> PresenceDiscoveryInfo {
        PresenceDiscoveryInfo::new(self.discovery_instance(), self.listen_port)
    }

    pub fn rotate_discovery(&self) -> Result<PresenceDiscoveryInfo, PresenceAgentError> {
        let nonce = random_bytes::<SESSION_DNS_SD_INSTANCE_NONCE_LEN>()
            .map_err(|_| PresenceAgentError::Random)?;
        let instance = session_dns_sd_instance(nonce);
        {
            let mut current = self
                .discovery_instance
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *current = instance.clone();
        }
        Ok(PresenceDiscoveryInfo::new(instance, self.listen_port))
    }

    pub fn subscribe_status(&self) -> watch::Receiver<PresenceSnapshot> {
        self.status.clone()
    }

    pub fn permission_snapshot(&self) -> PermissionSnapshot {
        self.permissions.borrow().clone()
    }

    pub fn subscribe_permissions(&self) -> watch::Receiver<PermissionSnapshot> {
        self.permissions.clone()
    }

    pub fn replace_policy(&self, policy: PolicyState) -> Result<(), PresenceAgentError> {
        if policy.revision() <= self.policy_tx.borrow().revision() {
            return Err(PresenceAgentError::StalePolicy);
        }
        self.policy_tx
            .send(policy)
            .map_err(|_| PresenceAgentError::Closed)
    }

    pub async fn replace_policy_and_wait(
        &self,
        policy: PolicyState,
    ) -> Result<(), PresenceAgentError> {
        let target_revision = policy.revision();
        let mut permissions = self.subscribe_permissions();
        self.replace_policy(policy)?;

        loop {
            if permissions.borrow().policy_revision() >= target_revision {
                return Ok(());
            }
            permissions
                .changed()
                .await
                .map_err(|_| PresenceAgentError::Closed)?;
        }
    }

    pub async fn fail_closed_policy(&self) -> Result<(), PresenceAgentError> {
        let revision = self
            .policy_tx
            .borrow()
            .revision()
            .checked_add(1)
            .ok_or(PresenceAgentError::PolicyRevisionExhausted)?;
        let policy = PolicyState::from_snapshot(revision, std::iter::empty())
            .expect("empty policy snapshot cannot contain duplicate rules");
        self.replace_policy_and_wait(policy).await
    }

    pub fn candidate_available(
        &self,
        route: TrustedSessionRoute,
    ) -> Result<(), PresenceAgentError> {
        self.send(AgentCommand::CandidateAvailable(route))
    }

    pub fn candidate_lost(&self, instance: String) -> Result<(), PresenceAgentError> {
        self.send(AgentCommand::CandidateLost(instance))
    }

    pub fn network_lost(&self) -> Result<(), PresenceAgentError> {
        self.send(AgentCommand::NetworkLost)
    }

    pub fn network_available(&self) -> Result<(), PresenceAgentError> {
        self.send(AgentCommand::NetworkAvailable)
    }

    pub fn disconnect(&self) -> Result<(), PresenceAgentError> {
        self.send(AgentCommand::Disconnect)
    }

    pub fn reconnect(&self) -> Result<(), PresenceAgentError> {
        self.send(AgentCommand::Reconnect)
    }

    pub fn take_clipboard_requests(
        &self,
    ) -> Result<mpsc::Receiver<ClipboardRequest>, ClipboardOperationError> {
        self.clipboard_requests
            .lock()
            .map_err(|_| ClipboardOperationError::Closed)?
            .take()
            .ok_or(ClipboardOperationError::Closed)
    }

    pub async fn send_clipboard_text(&self, text: String) -> Result<(), ClipboardOperationError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(AgentCommand::ClipboardWrite {
                text,
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClipboardOperationError::Closed)?;
        reply_rx
            .await
            .map_err(|_| ClipboardOperationError::Closed)?
    }

    pub async fn fetch_clipboard_text(&self) -> Result<String, ClipboardOperationError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(AgentCommand::ClipboardRead { reply: reply_tx })
            .await
            .map_err(|_| ClipboardOperationError::Closed)?;
        reply_rx
            .await
            .map_err(|_| ClipboardOperationError::Closed)?
    }

    pub async fn complete_clipboard_read(
        &self,
        request_id: crosslab_protocol::RequestId,
        result: Result<String, ClipboardPlatformError>,
    ) -> Result<(), ClipboardOperationError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(AgentCommand::ClipboardReadComplete {
                request_id,
                result,
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClipboardOperationError::Closed)?;
        reply_rx
            .await
            .map_err(|_| ClipboardOperationError::Closed)?
    }

    pub async fn complete_clipboard_write(
        &self,
        request_id: crosslab_protocol::RequestId,
        result: Result<(), ClipboardPlatformError>,
    ) -> Result<(), ClipboardOperationError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(AgentCommand::ClipboardWriteComplete {
                request_id,
                result,
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClipboardOperationError::Closed)?;
        reply_rx
            .await
            .map_err(|_| ClipboardOperationError::Closed)?
    }

    fn discovery_instance(&self) -> String {
        self.discovery_instance
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn send(&self, command: AgentCommand) -> Result<(), PresenceAgentError> {
        self.command_tx
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => PresenceAgentError::CommandQueueFull,
                mpsc::error::TrySendError::Closed(_) => PresenceAgentError::Closed,
            })
    }
}

impl Drop for TrustedPresenceAgent {
    fn drop(&mut self) {
        let _ = self.command_tx.try_send(AgentCommand::Stop);
    }
}
