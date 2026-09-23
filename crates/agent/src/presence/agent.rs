use std::sync::mpsc as std_mpsc;
use std::{
    net::{Ipv6Addr, SocketAddr},
    sync::{Arc, RwLock},
    thread,
};

use crosslab_core::{SESSION_DNS_SD_INSTANCE_NONCE_LEN, session_dns_sd_instance};
use crosslab_crypto::{SigningProvider, random_bytes};
use crosslab_identity_store::ProductIdentityState;
use crosslab_policy::PolicyState;
use crosslab_transport_quic::{QuicTransportConfig, TrustedSessionQuicServer};
use tokio::sync::{mpsc, watch};

use super::{
    runner::{AgentCommand, AgentSecurity, run_agent},
    types::{
        PermissionSnapshot, PresenceAgentError, PresenceDiscoveryInfo, PresencePhase,
        PresenceSnapshot, TrustedSessionRoute,
    },
};

const COMMAND_CAPACITY: usize = 64;

pub struct TrustedPresenceAgent {
    discovery_instance: Arc<RwLock<String>>,
    listen_port: u16,
    command_tx: mpsc::Sender<AgentCommand>,
    status: watch::Receiver<PresenceSnapshot>,
    permissions: watch::Receiver<PermissionSnapshot>,
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
        let (status_tx, status) =
            watch::channel(PresenceSnapshot::new(PresencePhase::Discovering, None));
        let (permissions_tx, permissions) =
            watch::channel(PermissionSnapshot::from_policy(&policy));
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
                        command_rx,
                        status_tx,
                        permissions_tx,
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
            status,
            permissions,
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
        if policy.revision() <= self.permissions.borrow().policy_revision() {
            return Err(PresenceAgentError::StalePolicy);
        }
        self.send(AgentCommand::ReplacePolicy(policy))
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
