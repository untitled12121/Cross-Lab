use tokio::sync::watch;

use crosslab_runtime::RuntimeStatus;

#[cfg(feature = "development-provisioning")]
use crosslab_policy::{NetworkClass, PolicyState, TrustRecord};
#[cfg(feature = "development-provisioning")]
use crosslab_runtime::{RuntimeActor, RuntimeActorConfig, RuntimeActorSession, RuntimeNode};
#[cfg(feature = "development-provisioning")]
use crosslab_transport_quic::{
    AuthenticatedQuicSession, QuicSessionTimeouts,
    development::{DevelopmentProvisioning, DevelopmentQuicClient},
};
#[cfg(feature = "development-provisioning")]
use std::{num::NonZeroUsize, path::PathBuf, sync::Arc, thread, time::Duration};
#[cfg(feature = "development-provisioning")]
use tokio::sync::mpsc;

#[cfg(feature = "development-provisioning")]
const COMMAND_CAPACITY: usize = 8;
#[cfg(feature = "development-provisioning")]
const RUNTIME_CAPACITY: usize = 8;
#[cfg(feature = "development-provisioning")]
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

pub struct DesktopRuntimeController {
    status: watch::Receiver<Option<RuntimeStatus>>,
    #[cfg(feature = "development-provisioning")]
    command_tx: Option<mpsc::Sender<DesktopCommand>>,
}

impl DesktopRuntimeController {
    pub fn from_environment() -> Self {
        let (status_tx, status) = watch::channel(None);

        #[cfg(feature = "development-provisioning")]
        let command_tx = std::env::var_os("CROSSLAB_DEVELOPMENT_PROVISIONING")
            .map(PathBuf::from)
            .and_then(|path| spawn_development(path, status_tx).ok());

        #[cfg(not(feature = "development-provisioning"))]
        let _ = status_tx;

        Self {
            status,
            #[cfg(feature = "development-provisioning")]
            command_tx,
        }
    }

    pub fn subscribe_status(&self) -> watch::Receiver<Option<RuntimeStatus>> {
        self.status.clone()
    }

    #[cfg(feature = "development-provisioning")]
    pub fn try_reconnect(&self) -> bool {
        self.command_tx
            .as_ref()
            .is_some_and(|command_tx| command_tx.try_send(DesktopCommand::Reconnect).is_ok())
    }
}

impl Default for DesktopRuntimeController {
    fn default() -> Self {
        Self::from_environment()
    }
}

#[cfg(feature = "development-provisioning")]
impl Drop for DesktopRuntimeController {
    fn drop(&mut self) {
        if let Some(command_tx) = self.command_tx.as_ref() {
            let _ = command_tx.try_send(DesktopCommand::Stop);
        }
    }
}

#[cfg(feature = "development-provisioning")]
enum DesktopCommand {
    Reconnect,
    Stop,
}

#[cfg(feature = "development-provisioning")]
enum DesktopEvent {
    Command(Option<DesktopCommand>),
    StatusChanged,
    TransportClosed,
    ActorClosed,
}

#[cfg(feature = "development-provisioning")]
struct ConnectedRuntime {
    actor: RuntimeActor,
    status: watch::Receiver<RuntimeStatus>,
    closed: watch::Receiver<bool>,
}

#[cfg(feature = "development-provisioning")]
fn spawn_development(
    path: PathBuf,
    status_tx: watch::Sender<Option<RuntimeStatus>>,
) -> Result<mpsc::Sender<DesktopCommand>, ()> {
    let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
    thread::Builder::new()
        .name("crosslab-desktop-development".into())
        .spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            runtime.block_on(run_development(path, status_tx, command_rx));
        })
        .map_err(|_| ())?;
    Ok(command_tx)
}

#[cfg(feature = "development-provisioning")]
async fn run_development(
    path: PathBuf,
    status_tx: watch::Sender<Option<RuntimeStatus>>,
    mut command_rx: mpsc::Receiver<DesktopCommand>,
) {
    let Ok(provisioning) = DevelopmentProvisioning::from_path(path) else {
        return;
    };
    let Ok(client) = provisioning.into_client() else {
        return;
    };
    let mut connected = connect(&client, &status_tx).await;

    loop {
        let event = match connected.as_mut() {
            Some(connection) => {
                tokio::select! {
                    command = command_rx.recv() => DesktopEvent::Command(command),
                    changed = connection.status.changed() => {
                        if changed.is_ok() {
                            DesktopEvent::StatusChanged
                        } else {
                            DesktopEvent::ActorClosed
                        }
                    }
                    changed = connection.closed.changed() => {
                        if changed.is_ok() {
                            DesktopEvent::TransportClosed
                        } else {
                            DesktopEvent::ActorClosed
                        }
                    }
                }
            }
            None => DesktopEvent::Command(command_rx.recv().await),
        };

        match event {
            DesktopEvent::Command(Some(DesktopCommand::Reconnect)) => {
                if let Some(connection) = connected.as_mut() {
                    let _ = reconnect(&client, connection, &status_tx).await;
                } else {
                    connected = connect(&client, &status_tx).await;
                }
            }
            DesktopEvent::Command(Some(DesktopCommand::Stop))
            | DesktopEvent::Command(None)
            | DesktopEvent::ActorClosed => {
                stop_connected(connected.take(), &status_tx).await;
                return;
            }
            DesktopEvent::StatusChanged => {
                if let Some(connection) = connected.as_ref() {
                    status_tx.send_replace(Some(connection.status.borrow().clone()));
                }
            }
            DesktopEvent::TransportClosed => {
                if let Some(connection) = connected.as_mut()
                    && *connection.closed.borrow()
                {
                    let _ = network_lost(connection, &status_tx).await;
                }
            }
        }
    }
}

#[cfg(feature = "development-provisioning")]
async fn connect(
    client: &DevelopmentQuicClient,
    status_tx: &watch::Sender<Option<RuntimeStatus>>,
) -> Option<ConnectedRuntime> {
    let session = client.connect_authenticated(timeouts()).await.ok()?;
    let (actor_session, closed) = runtime_session(session, client.peer_trust())?;

    let mut actor = RuntimeActor::new(actor_config());
    actor.start(actor_session).ok()?;
    let status = actor.subscribe_status().ok()?;
    status_tx.send_replace(Some(status.borrow().clone()));

    Some(ConnectedRuntime {
        actor,
        status,
        closed,
    })
}

#[cfg(feature = "development-provisioning")]
async fn reconnect(
    client: &DevelopmentQuicClient,
    connected: &mut ConnectedRuntime,
    status_tx: &watch::Sender<Option<RuntimeStatus>>,
) -> Result<(), ()> {
    let session = client
        .connect_authenticated(timeouts())
        .await
        .map_err(|_| ())?;
    let (actor_session, closed) = runtime_session(session, client.peer_trust()).ok_or(())?;

    connected
        .actor
        .reconnect(actor_session)
        .await
        .map_err(|_| ())?;
    connected.closed = closed;
    connected.status.changed().await.map_err(|_| ())?;
    status_tx.send_replace(Some(connected.status.borrow().clone()));
    Ok(())
}

#[cfg(feature = "development-provisioning")]
async fn network_lost(
    connected: &mut ConnectedRuntime,
    status_tx: &watch::Sender<Option<RuntimeStatus>>,
) -> Result<(), ()> {
    connected.actor.try_network_lost().map_err(|_| ())?;
    connected.status.changed().await.map_err(|_| ())?;
    status_tx.send_replace(Some(connected.status.borrow().clone()));
    Ok(())
}

#[cfg(feature = "development-provisioning")]
async fn stop_connected(
    connected: Option<ConnectedRuntime>,
    status_tx: &watch::Sender<Option<RuntimeStatus>>,
) {
    if let Some(mut connection) = connected {
        let _ = connection.actor.stop().await;
    }
    status_tx.send_replace(None);
}

#[cfg(feature = "development-provisioning")]
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

#[cfg(feature = "development-provisioning")]
fn actor_config() -> RuntimeActorConfig {
    RuntimeActorConfig::new(NonZeroUsize::new(RUNTIME_CAPACITY).expect("capacity is non-zero"))
}

#[cfg(feature = "development-provisioning")]
fn timeouts() -> QuicSessionTimeouts {
    QuicSessionTimeouts::new(CONNECT_TIMEOUT, CONNECT_TIMEOUT)
}
