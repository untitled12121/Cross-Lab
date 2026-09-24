use std::{fmt, num::NonZeroUsize};

use crosslab_core::ControlReceiveError;
use crosslab_policy::{PolicyState, SessionId, TrustRecord};
use crosslab_protocol::{
    CapabilityAdvertisement, ControlRequest, ControlResponseResult, RequestId,
};
use tokio::{
    runtime::Handle,
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};

use crate::{NodeError, NodeEvent, RuntimeNode, RuntimeStatus, command::RuntimeCommand};

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
    ControlRejected,
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
            Self::ControlRejected => "runtime actor rejected control operation",
        })
    }
}

impl std::error::Error for RuntimeActorError {}

pub struct RuntimeActorSession {
    node: RuntimeNode<'static>,
    peer_trust: TrustRecord,
    session_id: SessionId,
    control_ready: Option<watch::Receiver<u64>>,
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
            control_ready: None,
        }
    }

    pub fn with_control_ready(mut self, control_ready: watch::Receiver<u64>) -> Self {
        self.control_ready = Some(control_ready);
        self
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

    fn send_capabilities(
        &mut self,
        advertisement: CapabilityAdvertisement,
    ) -> Result<(), RuntimeActorError> {
        self.node
            .send_capability_advertisement(advertisement)
            .map_err(|_| RuntimeActorError::ControlRejected)
    }

    fn send_request(&mut self, request: ControlRequest) -> Result<(), RuntimeActorError> {
        self.node
            .send_request(request)
            .map_err(|_| RuntimeActorError::ControlRejected)
    }

    fn send_response(
        &mut self,
        request_id: RequestId,
        result: ControlResponseResult,
    ) -> Result<(), RuntimeActorError> {
        self.node
            .send_response(request_id, result)
            .map_err(|_| RuntimeActorError::ControlRejected)
    }

    fn receive_one(&mut self) -> Result<NodeEvent, NodeError> {
        self.node.receive_one(&self.peer_trust)
    }

    fn take_control_ready(&mut self) -> Option<watch::Receiver<u64>> {
        self.control_ready.take()
    }

    fn shutdown(&mut self) {
        self.node.shutdown();
    }
}

pub struct RuntimeActor {
    config: RuntimeActorConfig,
    command_tx: Option<mpsc::Sender<RuntimeCommand>>,
    status_rx: Option<watch::Receiver<RuntimeStatus>>,
    event_rx: Option<mpsc::Receiver<NodeEvent>>,
    task: Option<JoinHandle<()>>,
}

impl RuntimeActor {
    pub const fn new(config: RuntimeActorConfig) -> Self {
        Self {
            config,
            command_tx: None,
            status_rx: None,
            event_rx: None,
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
        let (event_tx, event_rx) = mpsc::channel(self.config.command_capacity.get());
        let (status_tx, status_rx) = watch::channel(initial_status);
        let task = runtime.spawn(run_actor(session, command_rx, status_tx, event_tx));

        self.command_tx = Some(command_tx);
        self.status_rx = Some(status_rx);
        self.event_rx = Some(event_rx);
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

    pub fn take_events(&mut self) -> Result<mpsc::Receiver<NodeEvent>, RuntimeActorError> {
        self.event_rx.take().ok_or(RuntimeActorError::NotRunning)
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

    pub async fn send_capabilities(
        &self,
        advertisement: CapabilityAdvertisement,
    ) -> Result<(), RuntimeActorError> {
        let command_tx = self
            .command_tx
            .as_ref()
            .ok_or(RuntimeActorError::NotRunning)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(RuntimeCommand::SendCapabilities {
                advertisement,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeActorError::ActorClosed)?;
        reply_rx.await.map_err(|_| RuntimeActorError::ActorClosed)?
    }

    pub async fn send_request(&self, request: ControlRequest) -> Result<(), RuntimeActorError> {
        let command_tx = self
            .command_tx
            .as_ref()
            .ok_or(RuntimeActorError::NotRunning)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(RuntimeCommand::SendRequest {
                request,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeActorError::ActorClosed)?;
        reply_rx.await.map_err(|_| RuntimeActorError::ActorClosed)?
    }

    pub async fn send_response(
        &self,
        request_id: RequestId,
        result: ControlResponseResult,
    ) -> Result<(), RuntimeActorError> {
        let command_tx = self
            .command_tx
            .as_ref()
            .ok_or(RuntimeActorError::NotRunning)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(RuntimeCommand::SendResponse {
                request_id,
                result,
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

enum ActorInput {
    Command(Option<RuntimeCommand>),
    ControlReady(bool),
}

async fn run_actor(
    mut session: RuntimeActorSession,
    mut command_rx: mpsc::Receiver<RuntimeCommand>,
    status_tx: watch::Sender<RuntimeStatus>,
    event_tx: mpsc::Sender<NodeEvent>,
) {
    let mut control_ready = session.take_control_ready();
    if !drain_inbound(&mut session, &status_tx, &event_tx).await {
        return;
    }

    loop {
        let input = if let Some(ready) = control_ready.as_mut() {
            tokio::select! {
                command = command_rx.recv() => ActorInput::Command(command),
                changed = ready.changed() => ActorInput::ControlReady(changed.is_ok()),
            }
        } else {
            ActorInput::Command(command_rx.recv().await)
        };

        match input {
            ActorInput::Command(Some(command)) => match command {
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
                RuntimeCommand::SendCapabilities {
                    advertisement,
                    reply,
                } => {
                    let _ = reply.send(session.send_capabilities(advertisement));
                }
                RuntimeCommand::SendRequest { request, reply } => {
                    let _ = reply.send(session.send_request(request));
                }
                RuntimeCommand::SendResponse {
                    request_id,
                    result,
                    reply,
                } => {
                    let _ = reply.send(session.send_response(request_id, result));
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

                    let replacement_ready = replacement.take_control_ready();
                    session.shutdown();
                    session = replacement;
                    control_ready = replacement_ready;
                    status_tx.send_replace(session.status());
                    let _ = reply.send(Ok(()));

                    if !drain_inbound(&mut session, &status_tx, &event_tx).await {
                        return;
                    }
                }
                RuntimeCommand::Stop { reply } => {
                    session.shutdown();
                    status_tx.send_replace(session.status());
                    let _ = reply.send(());
                    return;
                }
            },
            ActorInput::Command(None) => {
                session.shutdown();
                status_tx.send_replace(session.status());
                return;
            }
            ActorInput::ControlReady(true) => {
                if !drain_inbound(&mut session, &status_tx, &event_tx).await {
                    session.shutdown();
                    status_tx.send_replace(session.status());
                    return;
                }
            }
            ActorInput::ControlReady(false) => {
                session.shutdown();
                status_tx.send_replace(session.status());
                return;
            }
        }
    }
}

async fn drain_inbound(
    session: &mut RuntimeActorSession,
    status_tx: &watch::Sender<RuntimeStatus>,
    event_tx: &mpsc::Sender<NodeEvent>,
) -> bool {
    loop {
        match session.receive_one() {
            Ok(NodeEvent::CapabilitiesUpdated) => {
                status_tx.send_replace(session.status());
            }
            Ok(event) => {
                let session_closed = matches!(event, NodeEvent::SessionClosed(_));
                if event_tx.send(event).await.is_err() {
                    session.shutdown();
                    status_tx.send_replace(session.status());
                    return false;
                }
                if session_closed {
                    status_tx.send_replace(session.status());
                    return false;
                }
            }
            Err(NodeError::Receive(ControlReceiveError::Empty)) => return true,
            Err(NodeError::Receive(ControlReceiveError::Closed)) => {
                status_tx.send_replace(session.status());
                return false;
            }
            Err(_) => {
                let status = session.status();
                let active = status.session_id().is_some();
                status_tx.send_replace(status);
                if !active {
                    return false;
                }
            }
        }
    }
}

fn map_try_send_error(error: mpsc::error::TrySendError<RuntimeCommand>) -> RuntimeActorError {
    match error {
        mpsc::error::TrySendError::Full(_) => RuntimeActorError::CommandQueueFull,
        mpsc::error::TrySendError::Closed(_) => RuntimeActorError::ActorClosed,
    }
}
