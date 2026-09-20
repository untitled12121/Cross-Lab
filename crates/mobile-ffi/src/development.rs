use std::{num::NonZeroUsize, sync::Arc, thread, time::Duration};

use crosslab_policy::{NetworkClass, PolicyState, TrustRecord};
use crosslab_runtime::{
    RuntimeActor, RuntimeActorConfig, RuntimeActorSession, RuntimeNode, RuntimeStatus,
};
use crosslab_transport_quic::{
    AuthenticatedQuicSession, QuicSessionTimeouts,
    development::{DevelopmentProvisioning, DevelopmentQuicClient},
};
use tokio::sync::{mpsc, watch};

use crate::{MobileRuntimeError, runtime::MobileRuntimePublisher};

const COMMAND_CAPACITY: usize = 8;
const RUNTIME_CAPACITY: usize = 8;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

enum DevelopmentCommand {
    NetworkLost,
    NetworkAvailable,
    Disconnect,
    Stop,
}

enum DevelopmentEvent {
    Command(Option<DevelopmentCommand>),
    StatusChanged,
    TransportClosed,
    ActorClosed,
}

pub(crate) struct DevelopmentClientHandle {
    command_tx: mpsc::Sender<DevelopmentCommand>,
}

impl DevelopmentClientHandle {
    pub(crate) fn spawn(
        provisioning: DevelopmentProvisioning,
        publisher: MobileRuntimePublisher,
    ) -> Result<Self, MobileRuntimeError> {
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
        thread::Builder::new()
            .name("crosslab-mobile-development".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                runtime.block_on(run(provisioning, publisher, command_rx));
            })
            .map_err(|_| MobileRuntimeError::StateUnavailable)?;

        Ok(Self { command_tx })
    }

    pub(crate) fn network_lost(&self) -> Result<(), MobileRuntimeError> {
        self.send(DevelopmentCommand::NetworkLost)
    }

    pub(crate) fn network_available(&self) -> Result<(), MobileRuntimeError> {
        self.send(DevelopmentCommand::NetworkAvailable)
    }

    pub(crate) fn disconnect(&self) -> Result<(), MobileRuntimeError> {
        self.send(DevelopmentCommand::Disconnect)
    }

    pub(crate) fn stop(&self) -> Result<(), MobileRuntimeError> {
        self.send(DevelopmentCommand::Stop)
    }

    fn send(&self, command: DevelopmentCommand) -> Result<(), MobileRuntimeError> {
        self.command_tx
            .try_send(command)
            .map_err(|_| MobileRuntimeError::StateUnavailable)
    }
}

impl Drop for DevelopmentClientHandle {
    fn drop(&mut self) {
        let _ = self.command_tx.try_send(DevelopmentCommand::Stop);
    }
}

struct ConnectedRuntime {
    actor: RuntimeActor,
    status: watch::Receiver<RuntimeStatus>,
    closed: watch::Receiver<bool>,
}

async fn run(
    provisioning: DevelopmentProvisioning,
    publisher: MobileRuntimePublisher,
    mut command_rx: mpsc::Receiver<DevelopmentCommand>,
) {
    let Ok(client) = provisioning.into_client() else {
        return;
    };
    let mut connected = connect(&client, &publisher).await;

    loop {
        let event = match connected.as_mut() {
            Some(connection) => {
                tokio::select! {
                    command = command_rx.recv() => DevelopmentEvent::Command(command),
                    changed = connection.status.changed() => {
                        if changed.is_ok() {
                            DevelopmentEvent::StatusChanged
                        } else {
                            DevelopmentEvent::ActorClosed
                        }
                    }
                    changed = connection.closed.changed() => {
                        if changed.is_ok() {
                            DevelopmentEvent::TransportClosed
                        } else {
                            DevelopmentEvent::ActorClosed
                        }
                    }
                }
            }
            None => DevelopmentEvent::Command(command_rx.recv().await),
        };

        match event {
            DevelopmentEvent::Command(Some(DevelopmentCommand::NetworkLost)) => {
                if let Some(connection) = connected.as_mut() {
                    let _ = network_lost(connection, &publisher).await;
                }
            }
            DevelopmentEvent::Command(Some(DevelopmentCommand::NetworkAvailable)) => {
                if let Some(connection) = connected.as_mut() {
                    let _ = reconnect(&client, connection, &publisher).await;
                } else {
                    connected = connect(&client, &publisher).await;
                }
            }
            DevelopmentEvent::Command(Some(DevelopmentCommand::Disconnect)) => {
                stop_connected(connected.take()).await;
            }
            DevelopmentEvent::Command(Some(DevelopmentCommand::Stop))
            | DevelopmentEvent::Command(None)
            | DevelopmentEvent::ActorClosed => {
                stop_connected(connected.take()).await;
                return;
            }
            DevelopmentEvent::StatusChanged => {
                if let Some(connection) = connected.as_ref() {
                    let _ = publisher.publish_runtime_status(&connection.status.borrow());
                }
            }
            DevelopmentEvent::TransportClosed => {
                if let Some(connection) = connected.as_mut()
                    && *connection.closed.borrow()
                {
                    let _ = network_lost(connection, &publisher).await;
                }
            }
        }
    }
}

async fn connect(
    client: &DevelopmentQuicClient,
    publisher: &MobileRuntimePublisher,
) -> Option<ConnectedRuntime> {
    let session = client.connect_authenticated(timeouts()).await.ok()?;
    let (actor_session, closed) = runtime_session(session, client.peer_trust())?;

    let mut actor = RuntimeActor::new(actor_config());
    actor.start(actor_session).ok()?;
    let status = actor.subscribe_status().ok()?;
    publisher.publish_runtime_status(&status.borrow()).ok()?;

    Some(ConnectedRuntime {
        actor,
        status,
        closed,
    })
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
    let (actor_session, closed) = runtime_session(session, client.peer_trust())
        .ok_or(MobileRuntimeError::StateUnavailable)?;

    connected
        .actor
        .reconnect(actor_session)
        .await
        .map_err(|_| MobileRuntimeError::StateUnavailable)?;
    connected.closed = closed;
    connected
        .status
        .changed()
        .await
        .map_err(|_| MobileRuntimeError::StateUnavailable)?;
    publisher.publish_runtime_status(&connected.status.borrow())
}

async fn network_lost(
    connected: &mut ConnectedRuntime,
    publisher: &MobileRuntimePublisher,
) -> Result<(), MobileRuntimeError> {
    connected
        .actor
        .try_network_lost()
        .map_err(|_| MobileRuntimeError::StateUnavailable)?;
    connected
        .status
        .changed()
        .await
        .map_err(|_| MobileRuntimeError::StateUnavailable)?;
    publisher.publish_runtime_status(&connected.status.borrow())
}

async fn stop_connected(connected: Option<ConnectedRuntime>) {
    if let Some(mut connection) = connected {
        let _ = connection.actor.stop().await;
    }
}

fn runtime_session(
    session: AuthenticatedQuicSession,
    peer_trust: TrustRecord,
) -> Option<(RuntimeActorSession, watch::Receiver<bool>)> {
    let (session, transport) = session.into_parts();
    let closed = transport.subscribe_closed();
    let node = RuntimeNode::new_owned(
        session,
        Arc::new(transport),
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(RUNTIME_CAPACITY)?,
    )
    .ok()?;

    Some((RuntimeActorSession::new(node, peer_trust), closed))
}

fn actor_config() -> RuntimeActorConfig {
    RuntimeActorConfig::new(NonZeroUsize::new(RUNTIME_CAPACITY).expect("capacity is non-zero"))
}

fn timeouts() -> QuicSessionTimeouts {
    QuicSessionTimeouts::new(CONNECT_TIMEOUT, CONNECT_TIMEOUT)
}
