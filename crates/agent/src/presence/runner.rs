use std::{
    collections::BTreeMap,
    future::pending,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    num::NonZeroUsize,
    sync::{Arc, RwLock},
    time::Duration,
};

use crosslab_core::{MAX_SESSION_DISCOVERY_CANDIDATES, should_initiate_session};
use crosslab_crypto::SigningProvider;
use crosslab_identity::{DeviceCredential, DeviceId, OwnerAuthorityState};
use crosslab_policy::{NetworkClass, PolicyState, TrustRecord};
use crosslab_protocol::{FeatureSet, ProtocolRange};
use crosslab_runtime::{
    NodeEvent, RuntimeActor, RuntimeActorConfig, RuntimeActorSession, RuntimeNode,
};
use crosslab_transport_quic::{
    AuthenticatedQuicSession, QuicSessionAuthConfig, QuicSessionError, QuicSessionTimeouts,
    QuicTransportConfig, TrustedSessionQuicClient, TrustedSessionQuicServer,
};
use tokio::{
    sync::{mpsc, watch},
    time::Instant,
};

use super::types::{PermissionSnapshot, PresencePhase, PresenceSnapshot, TrustedSessionRoute};

const RUNTIME_CAPACITY: usize = 8;
const CONNECT_RESULT_CAPACITY: usize = 2;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const ACCEPT_TIMEOUT: Duration = Duration::from_secs(30);
const AUTH_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_DELAYS: [Duration; 5] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(15),
];

pub(super) struct AgentSecurity {
    pub(super) authority: OwnerAuthorityState,
    pub(super) local_credential: DeviceCredential,
    pub(super) signer: Arc<dyn SigningProvider + Send + Sync>,
    pub(super) trusts: Vec<TrustRecord>,
}

impl AgentSecurity {
    fn auth_config(&self) -> QuicSessionAuthConfig<'_> {
        QuicSessionAuthConfig::new_with_peer_trusts(
            &self.authority,
            self.local_credential,
            self.signer.as_ref(),
            &self.trusts,
            protocol_ranges(),
            features(),
        )
    }

    fn peer_trust(&self, device_id: DeviceId) -> Option<TrustRecord> {
        self.trusts
            .iter()
            .copied()
            .find(|trust| trust.device_id() == device_id)
    }
}

pub(super) enum AgentCommand {
    CandidateAvailable(TrustedSessionRoute),
    CandidateLost(String),
    NetworkLost,
    NetworkAvailable,
    Disconnect,
    Reconnect,
    Stop,
}

pub(super) struct AgentChannels {
    policy_rx: watch::Receiver<PolicyState>,
    command_rx: mpsc::Receiver<AgentCommand>,
    status_tx: watch::Sender<PresenceSnapshot>,
    permissions_tx: watch::Sender<PermissionSnapshot>,
}

impl AgentChannels {
    pub(super) fn new(
        policy_rx: watch::Receiver<PolicyState>,
        command_rx: mpsc::Receiver<AgentCommand>,
        status_tx: watch::Sender<PresenceSnapshot>,
        permissions_tx: watch::Sender<PermissionSnapshot>,
    ) -> Self {
        Self {
            policy_rx,
            command_rx,
            status_tx,
            permissions_tx,
        }
    }
}

struct CandidateState {
    route: TrustedSessionRoute,
    failures: usize,
    retry_at: Instant,
}

struct ConnectedRuntime {
    actor: RuntimeActor,
    status: watch::Receiver<crosslab_runtime::RuntimeStatus>,
    events: mpsc::Receiver<NodeEvent>,
    closed: watch::Receiver<bool>,
    peer_id: DeviceId,
    reconnecting: bool,
    _outbound_client: Option<TrustedSessionQuicClient>,
}

struct OutboundSuccess {
    client: TrustedSessionQuicClient,
    session: AuthenticatedQuicSession,
}

struct ConnectResult {
    instance: String,
    result: Result<OutboundSuccess, ()>,
}

enum ConnectedEvent {
    Command(Option<AgentCommand>),
    PolicyChanged,
    PolicyClosed,
    Runtime(NodeEvent),
    StatusChanged,
    TransportClosed,
}

pub(super) async fn run_agent(
    server: TrustedSessionQuicServer,
    security: Arc<AgentSecurity>,
    discovery_instance: Arc<RwLock<String>>,
    mut policy: PolicyState,
    channels: AgentChannels,
) {
    let AgentChannels {
        mut policy_rx,
        mut command_rx,
        status_tx,
        permissions_tx,
    } = channels;
    let (connect_tx, mut connect_rx) = mpsc::channel(CONNECT_RESULT_CAPACITY);
    let mut candidates = BTreeMap::<String, CandidateState>::new();
    let mut connected: Option<ConnectedRuntime> = None;
    let mut connecting_instance: Option<String> = None;
    let mut network_available = true;
    let mut auto_connect = true;

    loop {
        let local_instance = current_instance(&discovery_instance);
        maybe_start_connect(
            &security,
            &local_instance,
            &candidates,
            connected.as_ref(),
            &mut connecting_instance,
            network_available,
            auto_connect,
            &connect_tx,
            &status_tx,
        );

        if connected
            .as_ref()
            .is_some_and(|connection| !connection.reconnecting)
        {
            let event = {
                let connection = connected.as_mut().expect("connected state checked above");
                tokio::select! {
                    command = command_rx.recv() => ConnectedEvent::Command(command),
                    changed = policy_rx.changed() => {
                        if changed.is_ok() {
                            ConnectedEvent::PolicyChanged
                        } else {
                            ConnectedEvent::PolicyClosed
                        }
                    }
                    changed = connection.status.changed() => {
                        if changed.is_ok() {
                            ConnectedEvent::StatusChanged
                        } else {
                            ConnectedEvent::TransportClosed
                        }
                    }
                    event = connection.events.recv() => {
                        event.map_or(ConnectedEvent::TransportClosed, ConnectedEvent::Runtime)
                    }
                    _ = connection.closed.changed() => ConnectedEvent::TransportClosed
                }
            };

            match event {
                ConnectedEvent::Command(command) => {
                    if handle_command(
                        command,
                        &local_instance,
                        &mut candidates,
                        &mut connected,
                        &mut network_available,
                        &mut auto_connect,
                        &status_tx,
                    )
                    .await
                    {
                        server.close();
                        return;
                    }
                }
                ConnectedEvent::PolicyChanged => {
                    apply_policy_update(
                        &mut policy_rx,
                        &mut policy,
                        &mut connected,
                        &permissions_tx,
                        &status_tx,
                    )
                    .await;
                }
                ConnectedEvent::PolicyClosed => {
                    stop_connected(connected.take()).await;
                    server.close();
                    return;
                }
                ConnectedEvent::Runtime(event) => {
                    if matches!(event, NodeEvent::SessionClosed(_)) {
                        mark_transport_lost(&mut connected, &status_tx).await;
                    }
                }
                ConnectedEvent::StatusChanged => {
                    if let Some(connection) = connected.as_ref() {
                        publish_runtime(&status_tx, PresencePhase::Online, connection);
                    }
                }
                ConnectedEvent::TransportClosed => {
                    mark_transport_lost(&mut connected, &status_tx).await;
                }
            }
            continue;
        }

        let auth = security.auth_config();
        let retry_deadline = next_retry_deadline(
            &local_instance,
            &candidates,
            connecting_instance.as_deref(),
            network_available,
            auto_connect,
        );

        tokio::select! {
            command = command_rx.recv() => {
                if handle_command(
                    command,
                    &local_instance,
                    &mut candidates,
                    &mut connected,
                    &mut network_available,
                    &mut auto_connect,
                    &status_tx,
                ).await {
                    server.close();
                    return;
                }
            }
            changed = policy_rx.changed() => {
                if changed.is_err() {
                    stop_connected(connected.take()).await;
                    server.close();
                    return;
                }
                apply_policy_update(
                    &mut policy_rx,
                    &mut policy,
                    &mut connected,
                    &permissions_tx,
                    &status_tx,
                ).await;
            }
            accepted = server.accept_authenticated(&auth, server_timeouts()),
                if network_available && auto_connect =>
            {
                if let Ok(session) = accepted {
                    if install_session(
                        session,
                        None,
                        &security,
                        &policy,
                        &mut connected,
                        &status_tx,
                    ).await.is_err() {
                        publish_failure(&status_tx, connected.as_ref());
                    }
                } else if !matches!(accepted, Err(QuicSessionError::Timeout)) {
                    publish_waiting(&status_tx, connected.as_ref(), network_available, auto_connect);
                }
            }
            result = connect_rx.recv() => {
                if let Some(result) = result {
                    handle_connect_result(
                        result,
                        &security,
                        &policy,
                        &mut candidates,
                        &mut connected,
                        &mut connecting_instance,
                        network_available,
                        auto_connect,
                        &status_tx,
                    ).await;
                }
            }
            _ = wait_retry(retry_deadline) => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn maybe_start_connect(
    security: &Arc<AgentSecurity>,
    local_instance: &str,
    candidates: &BTreeMap<String, CandidateState>,
    connected: Option<&ConnectedRuntime>,
    connecting_instance: &mut Option<String>,
    network_available: bool,
    auto_connect: bool,
    connect_tx: &mpsc::Sender<ConnectResult>,
    status_tx: &watch::Sender<PresenceSnapshot>,
) {
    if connecting_instance.is_some()
        || !network_available
        || !auto_connect
        || connected.is_some_and(|connection| !connection.reconnecting)
    {
        return;
    }

    let now = Instant::now();
    let Some(candidate) = candidates
        .values()
        .filter(|candidate| {
            should_initiate_session(local_instance, candidate.route.instance())
                && candidate.retry_at <= now
        })
        .min_by_key(|candidate| candidate.route.instance())
    else {
        return;
    };

    let route = candidate.route.clone();
    let instance = route.instance().to_owned();
    *connecting_instance = Some(instance.clone());
    let runtime = connected.map(|connection| connection.status.borrow().clone());
    status_tx.send_replace(PresenceSnapshot::new(
        if runtime.is_some() {
            PresencePhase::Reconnecting
        } else {
            PresencePhase::Connecting
        },
        runtime,
    ));

    let security = Arc::clone(security);
    let connect_tx = connect_tx.clone();
    tokio::task::spawn_local(async move {
        let result = connect_outbound(&security, route.address()).await;
        let _ = connect_tx.send(ConnectResult { instance, result }).await;
    });
}

async fn connect_outbound(
    security: &AgentSecurity,
    remote: SocketAddr,
) -> Result<OutboundSuccess, ()> {
    let bind = match remote.ip() {
        IpAddr::V4(_) => SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
        IpAddr::V6(_) => SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
    };
    let client =
        TrustedSessionQuicClient::bind(bind, QuicTransportConfig::default()).map_err(|_| ())?;
    let auth = security.auth_config();
    let session = client
        .connect_authenticated(remote, &auth, client_timeouts())
        .await
        .map_err(|_| ())?;
    Ok(OutboundSuccess { client, session })
}

#[allow(clippy::too_many_arguments)]
async fn handle_connect_result(
    result: ConnectResult,
    security: &AgentSecurity,
    policy: &PolicyState,
    candidates: &mut BTreeMap<String, CandidateState>,
    connected: &mut Option<ConnectedRuntime>,
    connecting_instance: &mut Option<String>,
    network_available: bool,
    auto_connect: bool,
    status_tx: &watch::Sender<PresenceSnapshot>,
) {
    if connecting_instance.as_deref() != Some(result.instance.as_str()) {
        return;
    }
    *connecting_instance = None;

    match result.result {
        Ok(success)
            if network_available
                && auto_connect
                && candidates.contains_key(&result.instance)
                && connected
                    .as_ref()
                    .is_none_or(|connection| connection.reconnecting) =>
        {
            if let Some(candidate) = candidates.get_mut(&result.instance) {
                candidate.failures = 0;
                candidate.retry_at = Instant::now();
            }
            if install_session(
                success.session,
                Some(success.client),
                security,
                policy,
                connected,
                status_tx,
            )
            .await
            .is_err()
            {
                mark_candidate_failure(candidates, &result.instance);
                publish_failure(status_tx, connected.as_ref());
            }
        }
        Ok(success) => {
            success.session.transport().shutdown().await;
        }
        Err(()) => {
            mark_candidate_failure(candidates, &result.instance);
            publish_waiting(
                status_tx,
                connected.as_ref(),
                network_available,
                auto_connect,
            );
        }
    }
}

async fn install_session(
    session: AuthenticatedQuicSession,
    outbound_client: Option<TrustedSessionQuicClient>,
    security: &AgentSecurity,
    policy: &PolicyState,
    connected: &mut Option<ConnectedRuntime>,
    status_tx: &watch::Sender<PresenceSnapshot>,
) -> Result<(), ()> {
    let peer_id = session
        .session()
        .context()
        .map(|context| context.peer_device_id())
        .ok_or(())?;
    let peer_trust = security.peer_trust(peer_id).ok_or(())?;
    let (actor_session, closed) = runtime_session(session, peer_trust, policy).ok_or(())?;

    if let Some(connection) = connected.as_mut()
        && connection.reconnecting
        && connection.peer_id == peer_id
    {
        connection
            .actor
            .reconnect(actor_session)
            .await
            .map_err(|_| ())?;
        connection.closed = closed;
        connection._outbound_client = outbound_client;
        connection.reconnecting = false;
        let _ = connection.status.changed().await;
        publish_runtime(status_tx, PresencePhase::Online, connection);
        return Ok(());
    }

    if connected.is_some() {
        stop_connected(connected.take()).await;
    }

    let mut actor = RuntimeActor::new(actor_config());
    actor.start(actor_session).map_err(|_| ())?;
    let status = actor.subscribe_status().map_err(|_| ())?;
    let events = actor.take_events().map_err(|_| ())?;
    let connection = ConnectedRuntime {
        actor,
        status,
        events,
        closed,
        peer_id,
        reconnecting: false,
        _outbound_client: outbound_client,
    };
    publish_runtime(status_tx, PresencePhase::Online, &connection);
    *connected = Some(connection);
    Ok(())
}

async fn mark_transport_lost(
    connected: &mut Option<ConnectedRuntime>,
    status_tx: &watch::Sender<PresenceSnapshot>,
) {
    let Some(connection) = connected.as_mut() else {
        return;
    };
    if connection.reconnecting {
        return;
    }

    let _ = connection.actor.try_network_lost();
    let _ = connection.status.changed().await;
    connection.reconnecting = true;
    publish_runtime(status_tx, PresencePhase::Reconnecting, connection);
}

#[allow(clippy::too_many_arguments)]
async fn handle_command(
    command: Option<AgentCommand>,
    local_instance: &str,
    candidates: &mut BTreeMap<String, CandidateState>,
    connected: &mut Option<ConnectedRuntime>,
    network_available: &mut bool,
    auto_connect: &mut bool,
    status_tx: &watch::Sender<PresenceSnapshot>,
) -> bool {
    match command {
        Some(AgentCommand::CandidateAvailable(route)) => {
            upsert_candidate(local_instance, candidates, route);
            false
        }
        Some(AgentCommand::CandidateLost(instance)) => {
            candidates.remove(&instance);
            false
        }
        Some(AgentCommand::NetworkLost) => {
            *network_available = false;
            candidates.clear();
            mark_transport_lost(connected, status_tx).await;
            publish_waiting(status_tx, connected.as_ref(), false, *auto_connect);
            false
        }
        Some(AgentCommand::NetworkAvailable) => {
            *network_available = true;
            publish_waiting(status_tx, connected.as_ref(), true, *auto_connect);
            false
        }
        Some(AgentCommand::Disconnect) => {
            *auto_connect = false;
            candidates.clear();
            stop_connected(connected.take()).await;
            status_tx.send_replace(PresenceSnapshot::new(PresencePhase::Paused, None));
            false
        }
        Some(AgentCommand::Reconnect) => {
            *auto_connect = true;
            publish_waiting(status_tx, connected.as_ref(), *network_available, true);
            false
        }
        Some(AgentCommand::Stop) | None => {
            stop_connected(connected.take()).await;
            true
        }
    }
}

async fn apply_policy_update(
    policy_rx: &mut watch::Receiver<PolicyState>,
    policy: &mut PolicyState,
    connected: &mut Option<ConnectedRuntime>,
    permissions_tx: &watch::Sender<PermissionSnapshot>,
    status_tx: &watch::Sender<PresenceSnapshot>,
) {
    let next = policy_rx.borrow_and_update().clone();
    if *policy == next || next.revision() <= policy.revision() {
        return;
    }

    let apply_failed = match connected.as_ref() {
        Some(connection) => connection.actor.replace_policy(next.clone()).await.is_err(),
        None => false,
    };
    if apply_failed {
        stop_connected(connected.take()).await;
    }

    *policy = next;
    permissions_tx.send_replace(PermissionSnapshot::from_policy(policy));
    if apply_failed {
        publish_failure(status_tx, None);
    }
}

fn current_instance(discovery_instance: &RwLock<String>) -> String {
    discovery_instance
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn upsert_candidate(
    local_instance: &str,
    candidates: &mut BTreeMap<String, CandidateState>,
    route: TrustedSessionRoute,
) {
    if route.instance() == local_instance {
        return;
    }

    let instance = route.instance().to_owned();
    if let Some(candidate) = candidates.get_mut(&instance) {
        if candidate.route.address() != route.address() {
            candidate.route = route;
            candidate.failures = 0;
            candidate.retry_at = Instant::now();
        }
        return;
    }

    if candidates.len() >= MAX_SESSION_DISCOVERY_CANDIDATES {
        return;
    }

    candidates.insert(
        instance,
        CandidateState {
            route,
            failures: 0,
            retry_at: Instant::now(),
        },
    );
}

fn publish_waiting(
    status_tx: &watch::Sender<PresenceSnapshot>,
    connected: Option<&ConnectedRuntime>,
    network_available: bool,
    auto_connect: bool,
) {
    let runtime = connected.map(|connection| connection.status.borrow().clone());
    let phase = if !auto_connect {
        PresencePhase::Paused
    } else if runtime.is_some() || !network_available {
        PresencePhase::Reconnecting
    } else {
        PresencePhase::Discovering
    };
    status_tx.send_replace(PresenceSnapshot::new(phase, runtime));
}

fn publish_failure(
    status_tx: &watch::Sender<PresenceSnapshot>,
    connected: Option<&ConnectedRuntime>,
) {
    let runtime = connected.map(|connection| connection.status.borrow().clone());
    status_tx.send_replace(PresenceSnapshot::new(PresencePhase::Failed, runtime));
}

fn publish_runtime(
    status_tx: &watch::Sender<PresenceSnapshot>,
    phase: PresencePhase,
    connection: &ConnectedRuntime,
) {
    status_tx.send_replace(PresenceSnapshot::new(
        phase,
        Some(connection.status.borrow().clone()),
    ));
}

fn mark_candidate_failure(candidates: &mut BTreeMap<String, CandidateState>, instance: &str) {
    let Some(candidate) = candidates.get_mut(instance) else {
        return;
    };
    candidate.failures = candidate.failures.saturating_add(1);
    candidate.retry_at = Instant::now() + retry_delay(candidate.failures);
}

fn next_retry_deadline(
    local_instance: &str,
    candidates: &BTreeMap<String, CandidateState>,
    connecting_instance: Option<&str>,
    network_available: bool,
    auto_connect: bool,
) -> Option<Instant> {
    if !network_available || !auto_connect || connecting_instance.is_some() {
        return None;
    }
    candidates
        .values()
        .filter(|candidate| should_initiate_session(local_instance, candidate.route.instance()))
        .map(|candidate| candidate.retry_at)
        .min()
}

async fn wait_retry(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => pending::<()>().await,
    }
}

fn retry_delay(failures: usize) -> Duration {
    let index = failures.saturating_sub(1).min(RETRY_DELAYS.len() - 1);
    RETRY_DELAYS[index]
}

fn runtime_session(
    session: AuthenticatedQuicSession,
    peer_trust: TrustRecord,
    policy: &PolicyState,
) -> Option<(RuntimeActorSession, watch::Receiver<bool>)> {
    let (session, transport) = session.into_parts();
    let closed = transport.subscribe_closed();
    let control_ready = transport.subscribe_control_ready();
    let node = RuntimeNode::new_owned(
        session,
        Arc::new(transport),
        policy.clone(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(RUNTIME_CAPACITY)?,
    )
    .ok()?;
    Some((
        RuntimeActorSession::new(node, peer_trust).with_control_ready(control_ready),
        closed,
    ))
}

async fn stop_connected(connected: Option<ConnectedRuntime>) {
    if let Some(mut connection) = connected {
        let _ = connection.actor.stop().await;
    }
}

fn actor_config() -> RuntimeActorConfig {
    RuntimeActorConfig::new(NonZeroUsize::new(RUNTIME_CAPACITY).expect("capacity is non-zero"))
}

fn protocol_ranges() -> Vec<ProtocolRange> {
    vec![ProtocolRange::new(1, 0, 0).expect("protocol v1.0 range is valid")]
}

fn features() -> FeatureSet {
    FeatureSet::new(&[], &[]).expect("empty product feature profile is valid")
}

fn client_timeouts() -> QuicSessionTimeouts {
    QuicSessionTimeouts::new(CONNECT_TIMEOUT, AUTH_TIMEOUT)
}

fn server_timeouts() -> QuicSessionTimeouts {
    QuicSessionTimeouts::new(ACCEPT_TIMEOUT, AUTH_TIMEOUT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_set_rejects_self_and_stays_bounded() {
        let local = "s-00000000000000000000000000000000";
        let mut candidates = BTreeMap::new();

        let self_route =
            TrustedSessionRoute::new(local.to_owned(), SocketAddr::from(([127, 0, 0, 1], 4100)))
                .unwrap();
        upsert_candidate(local, &mut candidates, self_route);
        assert!(candidates.is_empty());

        for index in 1..=(MAX_SESSION_DISCOVERY_CANDIDATES + 4) {
            let instance = format!("s-{index:032x}");
            let route = TrustedSessionRoute::new(
                instance,
                SocketAddr::from(([127, 0, 0, 1], 4100 + index as u16)),
            )
            .unwrap();
            upsert_candidate(local, &mut candidates, route);
        }

        assert_eq!(candidates.len(), MAX_SESSION_DISCOVERY_CANDIDATES);
    }

    #[test]
    fn reconnect_backoff_caps_at_fifteen_seconds() {
        assert_eq!(retry_delay(1), Duration::from_secs(1));
        assert_eq!(retry_delay(2), Duration::from_secs(2));
        assert_eq!(retry_delay(3), Duration::from_secs(4));
        assert_eq!(retry_delay(4), Duration::from_secs(8));
        assert_eq!(retry_delay(5), Duration::from_secs(15));
        assert_eq!(retry_delay(99), Duration::from_secs(15));
    }
}
