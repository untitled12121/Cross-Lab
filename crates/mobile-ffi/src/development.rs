use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    },
    thread,
    time::Duration,
};

use crosslab_policy::{NetworkClass, PolicyState};
use crosslab_runtime::{RuntimeActor, RuntimeActorConfig, RuntimeActorSession, RuntimeNode};
use crosslab_transport_quic::{
    AuthenticatedQuicSession, QuicSessionTimeouts,
    development::{DevelopmentProvisioning, DevelopmentQuicClient},
};

use crate::{MobileRuntimeError, runtime::MobileRuntimePublisher};

const COMMAND_CAPACITY: usize = 8;
const RUNTIME_CAPACITY: usize = 8;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

enum DevelopmentCommand {
    NetworkLost,
    NetworkAvailable,
    Stop,
}

pub(crate) struct DevelopmentClientHandle {
    command_tx: SyncSender<DevelopmentCommand>,
}

impl DevelopmentClientHandle {
    pub(crate) fn spawn(
        provisioning: DevelopmentProvisioning,
        publisher: MobileRuntimePublisher,
    ) -> Result<Self, MobileRuntimeError> {
        let (command_tx, command_rx) = sync_channel(COMMAND_CAPACITY);
        thread::Builder::new()
            .name("crosslab-mobile-development".into())
            .spawn(move || run(provisioning, publisher, command_rx))
            .map_err(|_| MobileRuntimeError::StateUnavailable)?;

        Ok(Self { command_tx })
    }

    pub(crate) fn network_lost(&self) -> Result<(), MobileRuntimeError> {
        self.send(DevelopmentCommand::NetworkLost)
    }

    pub(crate) fn network_available(&self) -> Result<(), MobileRuntimeError> {
        self.send(DevelopmentCommand::NetworkAvailable)
    }

    pub(crate) fn stop(&self) -> Result<(), MobileRuntimeError> {
        self.send(DevelopmentCommand::Stop)
    }

    fn send(&self, command: DevelopmentCommand) -> Result<(), MobileRuntimeError> {
        self.command_tx
            .try_send(command)
            .map_err(|error| match error {
                TrySendError::Full(_) | TrySendError::Disconnected(_) => {
                    MobileRuntimeError::StateUnavailable
                }
            })
    }
}

impl Drop for DevelopmentClientHandle {
    fn drop(&mut self) {
        let _ = self.command_tx.try_send(DevelopmentCommand::Stop);
    }
}

struct ConnectedRuntime {
    actor: RuntimeActor,
    status: tokio::sync::watch::Receiver<crosslab_runtime::RuntimeStatus>,
}

fn run(
    provisioning: DevelopmentProvisioning,
    publisher: MobileRuntimePublisher,
    command_rx: Receiver<DevelopmentCommand>,
) {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };

    let client = {
        let _guard = runtime.enter();
        provisioning.into_client()
    };
    let Ok(client) = client else {
        return;
    };

    let mut connected = runtime.block_on(connect(&client, &publisher));

    while let Ok(command) = command_rx.recv() {
        match command {
            DevelopmentCommand::NetworkLost => {
                if let Some(connection) = connected.as_mut() {
                    let _ = network_lost(&runtime, connection, &publisher);
                }
            }
            DevelopmentCommand::NetworkAvailable => {
                if let Some(connection) = connected.as_mut() {
                    let _ = runtime.block_on(reconnect(&client, connection, &publisher));
                } else {
                    connected = runtime.block_on(connect(&client, &publisher));
                }
            }
            DevelopmentCommand::Stop => {
                if let Some(mut connection) = connected.take() {
                    let _ = runtime.block_on(connection.actor.stop());
                }
                return;
            }
        }
    }

    if let Some(mut connection) = connected {
        let _ = runtime.block_on(connection.actor.stop());
    }
}

async fn connect(
    client: &DevelopmentQuicClient,
    publisher: &MobileRuntimePublisher,
) -> Option<ConnectedRuntime> {
    let session = client.connect_authenticated(timeouts()).await.ok()?;
    let actor_session = runtime_session(session, client.peer_trust())?;

    let mut actor = RuntimeActor::new(actor_config());
    actor.start(actor_session).ok()?;
    let status = actor.subscribe_status().ok()?;
    publisher.publish_runtime_status(&status.borrow()).ok()?;

    Some(ConnectedRuntime { actor, status })
}

async fn reconnect(
    client: &DevelopmentQuicClient,
    connected: &mut ConnectedRuntime,
    publisher: &MobileRuntimePublisher,
) -> Result<(), MobileRuntimeError> {
    let session = client
        .connect_authenticated(timeouts())
        .await
        .map_err(|_| MobileRuntimeError::StateUnavailable)?;
    let actor_session = runtime_session(session, client.peer_trust())
        .ok_or(MobileRuntimeError::StateUnavailable)?;

    connected
        .actor
        .reconnect(actor_session)
        .await
        .map_err(|_| MobileRuntimeError::StateUnavailable)?;
    connected
        .status
        .changed()
        .await
        .map_err(|_| MobileRuntimeError::StateUnavailable)?;
    publisher.publish_runtime_status(&connected.status.borrow())
}

fn network_lost(
    runtime: &tokio::runtime::Runtime,
    connected: &mut ConnectedRuntime,
    publisher: &MobileRuntimePublisher,
) -> Result<(), MobileRuntimeError> {
    connected
        .actor
        .try_network_lost()
        .map_err(|_| MobileRuntimeError::StateUnavailable)?;
    runtime
        .block_on(connected.status.changed())
        .map_err(|_| MobileRuntimeError::StateUnavailable)?;
    publisher.publish_runtime_status(&connected.status.borrow())
}

fn runtime_session(
    session: AuthenticatedQuicSession,
    peer_trust: crosslab_policy::TrustRecord,
) -> Option<RuntimeActorSession> {
    let (session, transport) = session.into_parts();
    let node = RuntimeNode::new_owned(
        session,
        Arc::new(transport),
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(RUNTIME_CAPACITY)?,
    )
    .ok()?;
    Some(RuntimeActorSession::new(node, peer_trust))
}

fn actor_config() -> RuntimeActorConfig {
    RuntimeActorConfig::new(NonZeroUsize::new(RUNTIME_CAPACITY).expect("capacity is non-zero"))
}

fn timeouts() -> QuicSessionTimeouts {
    QuicSessionTimeouts::new(CONNECT_TIMEOUT, CONNECT_TIMEOUT)
}
