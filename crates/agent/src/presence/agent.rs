use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    thread,
};

use crosslab_core::{SESSION_DNS_SD_INSTANCE_NONCE_LEN, session_dns_sd_instance};
use crosslab_crypto::{SigningProvider, random_bytes};
use crosslab_identity_store::ProductIdentityState;
use crosslab_transport_quic::{QuicTransportConfig, TrustedSessionQuicServer};
use tokio::sync::{mpsc, watch};

use super::{
    runner::{AgentCommand, AgentSecurity, run_agent},
    types::{
        PresenceAgentError, PresenceDiscoveryInfo, PresencePhase, PresenceSnapshot,
        TrustedSessionRoute,
    },
};

const COMMAND_CAPACITY: usize = 64;

pub struct TrustedPresenceAgent {
    discovery: PresenceDiscoveryInfo,
    command_tx: mpsc::Sender<AgentCommand>,
    status: watch::Receiver<PresenceSnapshot>,
}

impl TrustedPresenceAgent {
    pub fn spawn(
        identity: ProductIdentityState,
        signer: Arc<dyn SigningProvider + Send + Sync>,
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
        let server = TrustedSessionQuicServer::bind(
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
            QuicTransportConfig::default(),
        )
        .map_err(|_| PresenceAgentError::Bind)?;
        let listen_port = server
            .local_addr()
            .map_err(|_| PresenceAgentError::Bind)?
            .port();

        let security = Arc::new(AgentSecurity {
            authority,
            local_credential: identity.local_credential(),
            signer,
            trusts,
        });
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
        let (status_tx, status) =
            watch::channel(PresenceSnapshot::new(PresencePhase::Discovering, None));
        let runner_instance = instance.clone();

        thread::Builder::new()
            .name("crosslab-presence-agent".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    status_tx.send_replace(PresenceSnapshot::new(PresencePhase::Failed, None));
                    return;
                };
                runtime.block_on(run_agent(
                    server,
                    security,
                    runner_instance,
                    command_rx,
                    status_tx,
                ));
            })
            .map_err(|_| PresenceAgentError::Thread)?;

        Ok(Self {
            discovery: PresenceDiscoveryInfo::new(instance, listen_port),
            command_tx,
            status,
        })
    }

    pub const fn discovery(&self) -> &PresenceDiscoveryInfo {
        &self.discovery
    }

    pub fn subscribe_status(&self) -> watch::Receiver<PresenceSnapshot> {
        self.status.clone()
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

    fn send(&self, command: AgentCommand) -> Result<(), PresenceAgentError> {
        self.command_tx.try_send(command).map_err(|error| match error {
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
