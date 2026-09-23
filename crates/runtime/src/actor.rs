use std::{fmt, num::NonZeroUsize};

use crosslab_policy::{PolicyState, SessionId, TrustRecord};
use tokio::{
    runtime::Handle,
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};

use crate::{RuntimeNode, RuntimeStatus, command::RuntimeCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeActorConfig {
    command_capacity: NonZeroUsize,
}

impl RuntimeActorConfig {
    pub const fn new(command_capacity: NonZeroUsize) -> Self {
        Self { command_capacity }
    }

    pub const fn command_capacity(self) -> NonZeroUsize {
        self.command_capacity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeActorError {
    AlreadyRunning,
    NotRunning,
    CommandQueueFull,
    StaleSession,
    ActorClosed,
    NoAsyncRuntime,
    TaskCancelled,
    PeerRevocationRejected,
    PolicyRejected,
}

impl fmt::Display for RuntimeActorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AlreadyRunning => "runtime actor is already running",
            Self::NotRunning => "runtime actor is not running",
            Self::CommandQueueFull => "runtime actor command queue is full",
            Self::StaleSession => "reconnect requires a fresh authenticated session",
            Self::ActorClosed => "runtime actor is closed",
            Self::NoAsyncRuntime => "runtime actor requires an active Tokio runtime",
            Self::TaskCancelled => "runtime actor task was cancelled",
            Self::PeerRevocationRejected => "runtime actor rejected peer revocation",
            Self::PolicyRejected => "runtime actor rejected stale policy state",
        })
    }
}

impl std::error::Error for RuntimeActorError {}

pub struct RuntimeActorSession {
    node: RuntimeNode<'static>,
    peer_trust: TrustRecord,
    session_id: SessionId,
}

impl RuntimeActorSession {
    pub fn new(node: RuntimeNode<'static>, peer_trust: TrustRecord) -> Self {
        let session_id = node
            .session()
            .context()
            .expect("runtime nodes have an authenticated session context")
            .session_id();
        Self {
            node,
            peer_trust,
            session_id,
        }
    }

    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn status(&self) -> RuntimeStatus {
        self.node.status(&self.peer_trust)
    }

    fn network_lost(&mut self) {
        self.node.network_lost();
    }

    fn revoke_peer(&mut self, peer_trust: TrustRecord) -> Result<(), RuntimeActorError> {
        self.node
            .apply_peer_revocation(&peer_trust)
            .map_err(|_| RuntimeActorError::PeerRevocationRejected)?;
        self.peer_trust = peer_trust;
        Ok(())
    }

    fn replace_policy(&mut self, policy: PolicyState) -> Result<bool, RuntimeActorError> {
        self.node
            .replace_policy(policy)
            .map_err(|_| RuntimeActorError::PolicyRejected)
    }

    fn shutdown(&mut self) {
        self.node.shutdown();
    }
}

pub struct RuntimeActor {
    config: RuntimeActorConfig,
    command_tx: Option<mpsc::Sender<RuntimeCommand>>,
    status_rx: Option<watch::Receiver<RuntimeStatus>>,
    task: Option<JoinHandle<()>>,
}

impl RuntimeActor {
    pub const fn new(config: RuntimeActorConfig) -> Self {
        Self {
            config,
            command_tx: None,
            status_rx: None,
            task: None,
        }
    }

    pub fn start(&mut self, session: RuntimeActorSession) -> Result<(), RuntimeActorError> {
        if self.task.is_some() {
            return Err(RuntimeActorError::AlreadyRunning);
        }

        let runtime = Handle::try_current().map_err(|_| RuntimeActorError::NoAsyncRuntime)?;
        let initial_status = session.status();
        let (command_tx, command_rx) = mpsc::channel(self.config.command_capacity.get());
        let (status_tx, status_rx) = watch::channel(initial_status);
        let task = runtime.spawn(run_actor(session, command_rx, status_tx));

        self.command_tx = Some(command_tx);
        self.status_rx = Some(status_rx);
        self.task = Some(task);
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.task.as_ref().is_some_and(|task| !task.is_finished())
    }

    pub fn subscribe_status(&self) -> Result<watch::Receiver<RuntimeStatus>, RuntimeActorError> {
        self.status_rx
            .as_ref()
            .cloned()
            .ok_or(RuntimeActorError::NotRunning)
    }

    pub fn try_network_lost(&self) -> Result<(), RuntimeActorError> {
        let command_tx = self
            .command_tx
            .as_ref()
            .ok_or(RuntimeActorError::NotRunning)?;
        command_tx
            .try_send(RuntimeCommand::NetworkLost)
            .map_err(map_try_send_error)
    }

    pub async fn revoke_peer(&self, peer_trust: TrustRecord) -> Result<(), RuntimeActorError> {
        let command_tx = self
            .command_tx
            .as_ref()
            .ok_or(RuntimeActorError::NotRunning)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(RuntimeCommand::PeerRevoked {
                peer_trust,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeActorError::ActorClosed)?;
        reply_rx.await.map_err(|_| RuntimeActorError::ActorClosed)?
    }

    pub async fn replace_policy(&self, policy: PolicyState) -> Result<bool, RuntimeActorError> {
        let command_tx = self
            .command_tx
            .as_ref()
            .ok_or(RuntimeActorError::NotRunning)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(RuntimeCommand::ReplacePolicy {
                policy,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeActorError::ActorClosed)?;
        reply_rx.await.map_err(|_| RuntimeActorError::ActorClosed)?
    }

    pub async fn reconnect(&self, session: RuntimeActorSession) -> Result<(), RuntimeActorError> {
        let command_tx = self
            .command_tx
            .as_ref()
            .ok_or(RuntimeActorError::NotRunning)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(RuntimeCommand::Reconnect {
                session: Box::new(session),
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeActorError::ActorClosed)?;
        reply_rx.await.map_err(|_| RuntimeActorError::ActorClosed)?
    }

    pub async fn stop(&mut self) -> Result<(), RuntimeActorError> {
        let command_tx = self
            .command_tx
            .as_ref()
            .ok_or(RuntimeActorError::NotRunning)?
            .clone();
        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(RuntimeCommand::Stop { reply: reply_tx })
            .await
            .map_err(|_| RuntimeActorError::ActorClosed)?;
        reply_rx.await.map_err(|_| RuntimeActorError::ActorClosed)?;

        self.command_tx = None;
        if let Some(task) = self.task.take() {
            task.await.map_err(|_| RuntimeActorError::TaskCancelled)?;
        }
        Ok(())
    }
}

impl Drop for RuntimeActor {
    fn drop(&mut self) {
        self.command_tx = None;
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn run_actor(
    mut session: RuntimeActorSession,
    mut command_rx: mpsc::Receiver<RuntimeCommand>,
    status_tx: watch::Sender<RuntimeStatus>,
) {
    while let Some(command) = command_rx.recv().await {
        match command {
            RuntimeCommand::NetworkLost => {
                session.network_lost();
                status_tx.send_replace(session.status());
            }
            RuntimeCommand::PeerRevoked { peer_trust, reply } => {
                let result = session.revoke_peer(peer_trust);
                status_tx.send_replace(session.status());
                let _ = reply.send(result);
            }
            RuntimeCommand::ReplacePolicy { policy, reply } => {
                let _ = reply.send(session.replace_policy(policy));
            }
            RuntimeCommand::Reconnect {
                session: replacement,
                reply,
            } => {
                let mut replacement = *replacement;
                if replacement.session_id() == session.session_id() {
                    replacement.shutdown();
                    let _ = reply.send(Err(RuntimeActorError::StaleSession));
                    continue;
                }

                session.shutdown();
                session = replacement;
                status_tx.send_replace(session.status());
                let _ = reply.send(Ok(()));
            }
            RuntimeCommand::Stop { reply } => {
                session.shutdown();
                status_tx.send_replace(session.status());
                let _ = reply.send(());
                return;
            }
        }
    }

    session.shutdown();
    status_tx.send_replace(session.status());
}

fn map_try_send_error(error: mpsc::error::TrySendError<RuntimeCommand>) -> RuntimeActorError {
    match error {
        mpsc::error::TrySendError::Full(_) => RuntimeActorError::CommandQueueFull,
        mpsc::error::TrySendError::Closed(_) => RuntimeActorError::ActorClosed,
    }
}
