use std::{
    collections::{BTreeMap, BTreeSet},
    future::pending,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    num::NonZeroUsize,
    sync::{Arc, RwLock},
    time::Duration,
};

use crosslab_core::{MAX_SESSION_DISCOVERY_CANDIDATES, should_initiate_session};
use crosslab_crypto::SigningProvider;
use crosslab_identity::{DeviceCredential, DeviceId, OwnerAuthorityState};
use crosslab_policy::{
    CapabilityId, CapabilityVersion, NetworkClass, OperationId, OperationName, PolicyState,
    TrustRecord, UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, FILE_TRANSFER_CAPABILITY_ID, FeatureSet, FileTransferAcceptance,
    FileTransferOffer, FileTransferResult, FileTransferTerminalOutcome, ProtocolErrorCode,
    ProtocolRange, RequestId, StreamId, TransferId,
};
use crosslab_runtime::{
    NodeEvent, RuntimeActor, RuntimeActorConfig, RuntimeActorSession, RuntimeNode,
    RuntimeStreamEvent,
};
use crosslab_transport_quic::{
    AuthenticatedQuicSession, QuicSessionAuthConfig, QuicSessionError, QuicSessionTimeouts,
    QuicTransportConfig, TrustedSessionQuicClient, TrustedSessionQuicServer,
};
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::Instant,
};

use super::types::{PermissionSnapshot, PresencePhase, PresenceSnapshot, TrustedSessionRoute};
use crate::{
    clipboard::{
        ClipboardAvailability, ClipboardKind, ClipboardOperationError, ClipboardPlatformError,
        ClipboardRequest, advertisement as clipboard_advertisement,
        capability_negotiated as clipboard_capability_negotiated,
        decode_inbound as decode_clipboard, decode_response as decode_clipboard_response,
        internal_failure as clipboard_internal_failure,
        local_capabilities as clipboard_local_capabilities,
        read_completion as clipboard_read_completion, read_request as clipboard_read_request,
        resource_failure as clipboard_resource_failure,
        write_completion as clipboard_write_completion, write_request as clipboard_write_request,
    },
    file_transfer::{
        FileTransferAvailability, FileTransferCancellation, FileTransferChunkError,
        FileTransferDataChunk, FileTransferDataEvent, FileTransferOperationError,
        FileTransferRequest, FileTransferSourceStream,
        advertisement as file_transfer_advertisement,
        already_complete_response as file_transfer_already_complete_response,
        cancelled_failure as file_transfer_cancelled_failure,
        capability_negotiated as file_transfer_capability_negotiated,
        decode_inbound as decode_file_transfer, decode_response as decode_file_transfer_response,
        decode_terminal_result_event as decode_file_transfer_terminal_result,
        internal_failure as file_transfer_internal_failure,
        local_capabilities as file_transfer_local_capabilities,
        map_chunk_error as map_file_transfer_chunk_error,
        offer_request as file_transfer_offer_request,
        ready_response as file_transfer_ready_response,
        resource_failure as file_transfer_resource_failure,
        result_subscription as file_transfer_result_subscription,
        source_stream_open as file_transfer_source_stream_open,
        terminal_result as file_transfer_terminal_result,
        terminal_result_event as file_transfer_terminal_result_event,
    },
};

const RUNTIME_CAPACITY: usize = 8;
const CONNECT_RESULT_CAPACITY: usize = 2;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const ACCEPT_TIMEOUT: Duration = Duration::from_secs(30);
const AUTH_TIMEOUT: Duration = Duration::from_secs(5);
const CLIPBOARD_OPERATION_TIMEOUT: Duration = Duration::from_secs(10);
const FILE_TRANSFER_OFFER_TIMEOUT: Duration = Duration::from_secs(30);
const FILE_TRANSFER_OPERATION_LIFETIME: Duration = Duration::from_secs(30);
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
    ClipboardWrite {
        text: String,
        reply: oneshot::Sender<Result<(), ClipboardOperationError>>,
    },
    ClipboardRead {
        reply: oneshot::Sender<Result<String, ClipboardOperationError>>,
    },
    ClipboardReadComplete {
        request_id: RequestId,
        result: Result<String, ClipboardPlatformError>,
        reply: oneshot::Sender<Result<(), ClipboardOperationError>>,
    },
    ClipboardWriteComplete {
        request_id: RequestId,
        result: Result<(), ClipboardPlatformError>,
        reply: oneshot::Sender<Result<(), ClipboardOperationError>>,
    },
    FileTransferOffer {
        offer: FileTransferOffer,
        reply: oneshot::Sender<Result<FileTransferAcceptance, FileTransferOperationError>>,
    },
    FileTransferCancelOffer {
        transfer_id: TransferId,
        reply: oneshot::Sender<Result<(), FileTransferOperationError>>,
    },
    FileTransferDecline {
        request_id: RequestId,
        reply: oneshot::Sender<Result<(), FileTransferOperationError>>,
    },
    FileTransferReady {
        request_id: RequestId,
        resume_offset: u64,
        reply: oneshot::Sender<Result<(), FileTransferOperationError>>,
    },
    FileTransferAlreadyComplete {
        request_id: RequestId,
        reply: oneshot::Sender<Result<(), FileTransferOperationError>>,
    },
    FileTransferOpen {
        transfer_id: TransferId,
        reply: oneshot::Sender<Result<FileTransferSourceStream, FileTransferOperationError>>,
    },
    FileTransferChunk {
        stream: FileTransferSourceStream,
        chunk: Vec<u8>,
        reply: oneshot::Sender<Result<(), FileTransferChunkError>>,
    },
    FileTransferFinish {
        stream: FileTransferSourceStream,
        reply: oneshot::Sender<Result<FileTransferResult, FileTransferOperationError>>,
    },
    FileTransferCancelSend {
        stream: FileTransferSourceStream,
        reply: oneshot::Sender<Result<FileTransferResult, FileTransferOperationError>>,
    },
    FileTransferRecoverClosed {
        stream: FileTransferSourceStream,
        reply: oneshot::Sender<Result<FileTransferResult, FileTransferOperationError>>,
    },
    FileTransferCancelReceive {
        transfer_id: TransferId,
        reply: oneshot::Sender<Result<Option<StreamId>, FileTransferOperationError>>,
    },
    FileTransferFailReceive {
        stream_id: StreamId,
        transfer_id: TransferId,
        outcome: FileTransferTerminalOutcome,
        reply: oneshot::Sender<Result<(), FileTransferOperationError>>,
    },
    FileTransferTerminal {
        transfer_id: TransferId,
        outcome: FileTransferTerminalOutcome,
        reply: oneshot::Sender<Result<(), FileTransferOperationError>>,
    },
    Stop,
}

#[derive(Clone, Copy)]
pub(super) struct RuntimeAvailability {
    clipboard: ClipboardAvailability,
    file_transfer: FileTransferAvailability,
}

impl RuntimeAvailability {
    pub(super) const fn new(
        clipboard: ClipboardAvailability,
        file_transfer: FileTransferAvailability,
    ) -> Self {
        Self {
            clipboard,
            file_transfer,
        }
    }
}

pub(super) struct CapabilityChannels {
    clipboard_requests_tx: mpsc::Sender<ClipboardRequest>,
    file_transfer_requests_tx: mpsc::Sender<FileTransferRequest>,
    file_transfer_cancellations_tx: mpsc::Sender<FileTransferCancellation>,
    file_transfer_data_tx: mpsc::Sender<FileTransferDataEvent>,
}

impl CapabilityChannels {
    pub(super) fn new(
        clipboard_requests_tx: mpsc::Sender<ClipboardRequest>,
        file_transfer_requests_tx: mpsc::Sender<FileTransferRequest>,
        file_transfer_cancellations_tx: mpsc::Sender<FileTransferCancellation>,
        file_transfer_data_tx: mpsc::Sender<FileTransferDataEvent>,
    ) -> Self {
        Self {
            clipboard_requests_tx,
            file_transfer_requests_tx,
            file_transfer_cancellations_tx,
            file_transfer_data_tx,
        }
    }
}

pub(super) struct AgentChannels {
    policy_rx: watch::Receiver<PolicyState>,
    command_rx: mpsc::Receiver<AgentCommand>,
    status_tx: watch::Sender<PresenceSnapshot>,
    permissions_tx: watch::Sender<PermissionSnapshot>,
    capabilities: CapabilityChannels,
    availability: RuntimeAvailability,
}

impl AgentChannels {
    pub(super) fn new(
        policy_rx: watch::Receiver<PolicyState>,
        command_rx: mpsc::Receiver<AgentCommand>,
        status_tx: watch::Sender<PresenceSnapshot>,
        permissions_tx: watch::Sender<PermissionSnapshot>,
        capabilities: CapabilityChannels,
        availability: RuntimeAvailability,
    ) -> Self {
        Self {
            policy_rx,
            command_rx,
            status_tx,
            permissions_tx,
            capabilities,
            availability,
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

enum PendingClipboard {
    Read {
        deadline: Instant,
        reply: oneshot::Sender<Result<String, ClipboardOperationError>>,
    },
    Write {
        deadline: Instant,
        reply: oneshot::Sender<Result<(), ClipboardOperationError>>,
    },
}

impl PendingClipboard {
    fn kind(&self) -> ClipboardKind {
        match self {
            Self::Read { .. } => ClipboardKind::Read,
            Self::Write { .. } => ClipboardKind::Write,
        }
    }

    fn deadline(&self) -> Instant {
        match self {
            Self::Read { deadline, .. } | Self::Write { deadline, .. } => *deadline,
        }
    }

    fn finish(self, result: Result<Option<String>, ClipboardOperationError>) {
        match self {
            Self::Read { reply, .. } => {
                let result =
                    result.and_then(|text| text.ok_or(ClipboardOperationError::InvalidResponse));
                let _ = reply.send(result);
            }
            Self::Write { reply, .. } => {
                let result = result.and_then(|text| {
                    if text.is_none() {
                        Ok(())
                    } else {
                        Err(ClipboardOperationError::InvalidResponse)
                    }
                });
                let _ = reply.send(result);
            }
        }
    }

    fn cancel(self, error: ClipboardOperationError) {
        match self {
            Self::Read { reply, .. } => {
                let _ = reply.send(Err(error));
            }
            Self::Write { reply, .. } => {
                let _ = reply.send(Err(error));
            }
        }
    }
}

struct ClipboardRuntimeState {
    requests_tx: mpsc::Sender<ClipboardRequest>,
    availability: ClipboardAvailability,
    outgoing: BTreeMap<RequestId, PendingClipboard>,
    inbound: BTreeMap<RequestId, ClipboardKind>,
}

impl ClipboardRuntimeState {
    fn new(
        requests_tx: mpsc::Sender<ClipboardRequest>,
        availability: ClipboardAvailability,
    ) -> Self {
        Self {
            requests_tx,
            availability,
            outgoing: BTreeMap::new(),
            inbound: BTreeMap::new(),
        }
    }

    fn cancel_all(&mut self, error: ClipboardOperationError) {
        for (_, pending) in core::mem::take(&mut self.outgoing) {
            pending.cancel(error);
        }
        self.inbound.clear();
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.outgoing.values().map(PendingClipboard::deadline).min()
    }
}

struct PendingFileOffer {
    offer: FileTransferOffer,
    deadline: Instant,
    reply: oneshot::Sender<Result<FileTransferAcceptance, FileTransferOperationError>>,
}

struct DestinationReadyFileTransfer {
    transfer_id: TransferId,
    operation_id: OperationId,
    resume_offset: u64,
    deadline: Instant,
}

#[derive(Clone, Copy)]
struct ActiveInboundFileTransfer {
    transfer_id: TransferId,
}

struct TerminalReadyFileTransfer {
    transfer_id: TransferId,
    stream_id: StreamId,
    deadline: Instant,
}

enum SourceFileTransfer {
    Ready {
        operation_id: OperationId,
        resume_offset: u64,
        deadline: Instant,
    },
    Streaming {
        stream_id: StreamId,
        resume_offset: u64,
    },
    AwaitingResult {
        deadline: Instant,
        reply: oneshot::Sender<Result<FileTransferResult, FileTransferOperationError>>,
    },
}

struct FileTransferRuntimeState {
    requests_tx: mpsc::Sender<FileTransferRequest>,
    cancellations_tx: mpsc::Sender<FileTransferCancellation>,
    data_tx: mpsc::Sender<FileTransferDataEvent>,
    availability: FileTransferAvailability,
    outgoing: BTreeMap<RequestId, PendingFileOffer>,
    inbound: BTreeMap<RequestId, FileTransferRequest>,
    destination_ready: Vec<DestinationReadyFileTransfer>,
    inbound_streams: BTreeMap<StreamId, ActiveInboundFileTransfer>,
    terminal_ready: Vec<TerminalReadyFileTransfer>,
    source: BTreeMap<TransferId, SourceFileTransfer>,
    completed_results: Vec<FileTransferResult>,
}

impl FileTransferRuntimeState {
    fn new(
        requests_tx: mpsc::Sender<FileTransferRequest>,
        cancellations_tx: mpsc::Sender<FileTransferCancellation>,
        data_tx: mpsc::Sender<FileTransferDataEvent>,
        availability: FileTransferAvailability,
    ) -> Self {
        Self {
            requests_tx,
            cancellations_tx,
            data_tx,
            availability,
            outgoing: BTreeMap::new(),
            inbound: BTreeMap::new(),
            destination_ready: Vec::with_capacity(RUNTIME_CAPACITY),
            inbound_streams: BTreeMap::new(),
            terminal_ready: Vec::with_capacity(RUNTIME_CAPACITY),
            source: BTreeMap::new(),
            completed_results: Vec::with_capacity(RUNTIME_CAPACITY),
        }
    }

    fn notify_request_cancelled(&self, request_id: RequestId, transfer_id: TransferId) {
        let _ = self
            .cancellations_tx
            .try_send(FileTransferCancellation::request(request_id, transfer_id));
    }

    fn notify_transfer_interrupted(&self, transfer_id: TransferId) {
        let _ = self
            .cancellations_tx
            .try_send(FileTransferCancellation::transfer(transfer_id));
    }

    fn cancel_all(&mut self, error: FileTransferOperationError) {
        for (_, pending) in core::mem::take(&mut self.outgoing) {
            let _ = pending.reply.send(Err(error));
        }
        for (_, source) in core::mem::take(&mut self.source) {
            if let SourceFileTransfer::AwaitingResult { reply, .. } = source {
                let _ = reply.send(Err(error));
            }
        }
        for (request_id, request) in core::mem::take(&mut self.inbound) {
            self.notify_request_cancelled(request_id, request.offer().transfer_id());
        }

        let mut interrupted = BTreeSet::new();
        interrupted.extend(
            self.destination_ready
                .drain(..)
                .map(|ready| ready.transfer_id),
        );
        interrupted.extend(
            core::mem::take(&mut self.inbound_streams)
                .into_values()
                .map(|active| active.transfer_id),
        );
        for transfer_id in interrupted {
            self.notify_transfer_interrupted(transfer_id);
        }

        self.terminal_ready.clear();
        self.completed_results.clear();
    }

    fn destination_capacity_used(&self) -> usize {
        self.inbound.len()
            + self.destination_ready.len()
            + self.inbound_streams.len()
            + self.terminal_ready.len()
    }

    fn cache_result(&mut self, result: FileTransferResult) {
        if self
            .completed_results
            .iter()
            .any(|current| current.transfer_id() == result.transfer_id())
        {
            return;
        }
        if self.completed_results.len() >= RUNTIME_CAPACITY {
            self.completed_results.remove(0);
        }
        self.completed_results.push(result);
    }

    fn take_cached_result(&mut self, transfer_id: TransferId) -> Option<FileTransferResult> {
        let position = self
            .completed_results
            .iter()
            .position(|result| result.transfer_id() == transfer_id)?;
        Some(self.completed_results.remove(position))
    }

    fn next_deadline(&self) -> Option<Instant> {
        let source_deadlines = self.source.values().filter_map(|source| match source {
            SourceFileTransfer::Ready { deadline, .. }
            | SourceFileTransfer::AwaitingResult { deadline, .. } => Some(*deadline),
            SourceFileTransfer::Streaming { .. } => None,
        });
        self.outgoing
            .values()
            .map(|pending| pending.deadline)
            .chain(self.destination_ready.iter().map(|ready| ready.deadline))
            .chain(self.terminal_ready.iter().map(|terminal| terminal.deadline))
            .chain(source_deadlines)
            .min()
    }
}

enum ConnectedEvent {
    Command(Option<AgentCommand>),
    PolicyChanged,
    PolicyClosed,
    Runtime(NodeEvent),
    ClipboardTimeout,
    FileTransferTimeout,
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
        capabilities,
        availability,
    } = channels;
    let CapabilityChannels {
        clipboard_requests_tx,
        file_transfer_requests_tx,
        file_transfer_cancellations_tx,
        file_transfer_data_tx,
    } = capabilities;
    let (connect_tx, mut connect_rx) = mpsc::channel(CONNECT_RESULT_CAPACITY);
    let mut clipboard = ClipboardRuntimeState::new(clipboard_requests_tx, availability.clipboard);
    let mut file_transfer = FileTransferRuntimeState::new(
        file_transfer_requests_tx,
        file_transfer_cancellations_tx,
        file_transfer_data_tx,
        availability.file_transfer,
    );
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
                    _ = wait_retry(clipboard.next_deadline()) => ConnectedEvent::ClipboardTimeout,
                    _ = wait_retry(file_transfer.next_deadline()) => ConnectedEvent::FileTransferTimeout,
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
                        &mut clipboard,
                        &mut file_transfer,
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
                        &mut clipboard,
                        &mut file_transfer,
                    )
                    .await;
                }
                ConnectedEvent::PolicyClosed => {
                    clipboard.cancel_all(ClipboardOperationError::Cancelled);
                    file_transfer.cancel_all(FileTransferOperationError::Cancelled);
                    stop_connected(connected.take()).await;
                    server.close();
                    return;
                }
                ConnectedEvent::Runtime(event) => {
                    let session_closed = matches!(event, NodeEvent::SessionClosed(_));
                    handle_runtime_event(event, &mut connected, &mut clipboard, &mut file_transfer)
                        .await;
                    if session_closed {
                        mark_transport_lost(
                            &mut connected,
                            &status_tx,
                            &mut clipboard,
                            &mut file_transfer,
                        )
                        .await;
                    }
                }
                ConnectedEvent::ClipboardTimeout => {
                    expire_clipboard_operations(&mut connected, &mut clipboard).await;
                }
                ConnectedEvent::FileTransferTimeout => {
                    expire_file_transfer_offers(&mut connected, &mut file_transfer).await;
                }
                ConnectedEvent::StatusChanged => {
                    if let Some(connection) = connected.as_ref() {
                        publish_runtime(&status_tx, PresencePhase::Online, connection);
                    }
                }
                ConnectedEvent::TransportClosed => {
                    mark_transport_lost(
                        &mut connected,
                        &status_tx,
                        &mut clipboard,
                        &mut file_transfer,
                    )
                    .await;
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
                    &mut clipboard,
                    &mut file_transfer,
                ).await {
                    server.close();
                    return;
                }
            }
            changed = policy_rx.changed() => {
                if changed.is_err() {
                    clipboard.cancel_all(ClipboardOperationError::Cancelled);
                    file_transfer.cancel_all(FileTransferOperationError::Cancelled);
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
                    &mut clipboard,
                    &mut file_transfer,
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
                        RuntimeAvailability::new(
                            clipboard.availability,
                            file_transfer.availability,
                        ),
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
                        RuntimeAvailability::new(
                            clipboard.availability,
                            file_transfer.availability,
                        ),
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
    availability: RuntimeAvailability,
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
                availability,
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
    availability: RuntimeAvailability,
) -> Result<(), ()> {
    let peer_id = session
        .session()
        .context()
        .map(|context| context.peer_device_id())
        .ok_or(())?;
    let peer_trust = security.peer_trust(peer_id).ok_or(())?;
    let (actor_session, closed) = runtime_session(
        session,
        peer_trust,
        policy,
        availability.clipboard,
        availability.file_transfer,
    )
    .ok_or(())?;

    if let Some(connection) = connected.as_mut()
        && connection.reconnecting
        && connection.peer_id == peer_id
    {
        connection
            .actor
            .reconnect(actor_session)
            .await
            .map_err(|_| ())?;
        connection
            .actor
            .send_capabilities(runtime_advertisement(
                availability.clipboard,
                availability.file_transfer,
            ))
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
    if actor
        .send_capabilities(runtime_advertisement(
            availability.clipboard,
            availability.file_transfer,
        ))
        .await
        .is_err()
    {
        let _ = actor.stop().await;
        return Err(());
    }
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
    clipboard: &mut ClipboardRuntimeState,
    file_transfer: &mut FileTransferRuntimeState,
) {
    clipboard.cancel_all(ClipboardOperationError::Cancelled);
    file_transfer.cancel_all(FileTransferOperationError::Cancelled);
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
    clipboard: &mut ClipboardRuntimeState,
    file_transfer: &mut FileTransferRuntimeState,
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
            mark_transport_lost(connected, status_tx, clipboard, file_transfer).await;
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
            clipboard.cancel_all(ClipboardOperationError::Cancelled);
            file_transfer.cancel_all(FileTransferOperationError::Cancelled);
            stop_connected(connected.take()).await;
            status_tx.send_replace(PresenceSnapshot::new(PresencePhase::Paused, None));
            false
        }
        Some(AgentCommand::Reconnect) => {
            *auto_connect = true;
            publish_waiting(status_tx, connected.as_ref(), *network_available, true);
            false
        }
        Some(AgentCommand::ClipboardWrite { text, reply }) => {
            start_clipboard_write(connected.as_ref(), clipboard, text, reply).await;
            false
        }
        Some(AgentCommand::ClipboardRead { reply }) => {
            start_clipboard_read(connected.as_ref(), clipboard, reply).await;
            false
        }
        Some(AgentCommand::ClipboardReadComplete {
            request_id,
            result,
            reply,
        }) => {
            let outcome =
                complete_clipboard_read(connected.as_ref(), clipboard, request_id, result).await;
            let _ = reply.send(outcome);
            false
        }
        Some(AgentCommand::ClipboardWriteComplete {
            request_id,
            result,
            reply,
        }) => {
            let outcome =
                complete_clipboard_write(connected.as_ref(), clipboard, request_id, result).await;
            let _ = reply.send(outcome);
            false
        }
        Some(AgentCommand::FileTransferOffer { offer, reply }) => {
            start_file_transfer_offer(connected.as_ref(), file_transfer, offer, reply).await;
            false
        }
        Some(AgentCommand::FileTransferCancelOffer { transfer_id, reply }) => {
            let outcome =
                cancel_file_transfer_offer(connected.as_ref(), file_transfer, transfer_id).await;
            let _ = reply.send(outcome);
            false
        }
        Some(AgentCommand::FileTransferDecline { request_id, reply }) => {
            let outcome =
                decline_file_transfer_request(connected.as_ref(), file_transfer, request_id).await;
            let _ = reply.send(outcome);
            false
        }
        Some(AgentCommand::FileTransferReady {
            request_id,
            resume_offset,
            reply,
        }) => {
            let outcome = complete_file_transfer_ready(
                connected.as_ref(),
                file_transfer,
                request_id,
                resume_offset,
            )
            .await;
            let _ = reply.send(outcome);
            false
        }
        Some(AgentCommand::FileTransferAlreadyComplete { request_id, reply }) => {
            let outcome = complete_file_transfer_already_complete(
                connected.as_ref(),
                file_transfer,
                request_id,
            )
            .await;
            let _ = reply.send(outcome);
            false
        }
        Some(AgentCommand::FileTransferOpen { transfer_id, reply }) => {
            let outcome =
                open_file_transfer_stream(connected.as_ref(), file_transfer, transfer_id).await;
            let _ = reply.send(outcome);
            false
        }
        Some(AgentCommand::FileTransferChunk {
            stream,
            chunk,
            reply,
        }) => {
            let outcome =
                send_file_transfer_chunk(connected.as_ref(), file_transfer, stream, chunk).await;
            let _ = reply.send(outcome);
            false
        }
        Some(AgentCommand::FileTransferFinish { stream, reply }) => {
            finish_file_transfer_stream(connected.as_ref(), file_transfer, stream, false, reply)
                .await;
            false
        }
        Some(AgentCommand::FileTransferCancelSend { stream, reply }) => {
            finish_file_transfer_stream(connected.as_ref(), file_transfer, stream, true, reply)
                .await;
            false
        }
        Some(AgentCommand::FileTransferRecoverClosed { stream, reply }) => {
            recover_closed_file_transfer_result(connected.as_ref(), file_transfer, stream, reply)
                .await;
            false
        }
        Some(AgentCommand::FileTransferCancelReceive { transfer_id, reply }) => {
            let outcome =
                cancel_file_transfer_receive(connected.as_ref(), file_transfer, transfer_id).await;
            let _ = reply.send(outcome);
            false
        }
        Some(AgentCommand::FileTransferFailReceive {
            stream_id,
            transfer_id,
            outcome,
            reply,
        }) => {
            let result = fail_file_transfer_receive(
                connected.as_ref(),
                file_transfer,
                stream_id,
                transfer_id,
                outcome,
            )
            .await;
            let _ = reply.send(result);
            false
        }
        Some(AgentCommand::FileTransferTerminal {
            transfer_id,
            outcome,
            reply,
        }) => {
            let result = complete_file_transfer_terminal(
                connected.as_ref(),
                file_transfer,
                transfer_id,
                outcome,
            )
            .await;
            let _ = reply.send(result);
            false
        }
        Some(AgentCommand::Stop) | None => {
            clipboard.cancel_all(ClipboardOperationError::Cancelled);
            file_transfer.cancel_all(FileTransferOperationError::Cancelled);
            stop_connected(connected.take()).await;
            true
        }
    }
}

async fn start_clipboard_write(
    connected: Option<&ConnectedRuntime>,
    clipboard: &mut ClipboardRuntimeState,
    text: String,
    reply: oneshot::Sender<Result<(), ClipboardOperationError>>,
) {
    if clipboard.outgoing.len() >= RUNTIME_CAPACITY {
        let _ = reply.send(Err(ClipboardOperationError::ResourceLimit));
        return;
    }
    let Some(connection) = connected.filter(|connection| !connection.reconnecting) else {
        let _ = reply.send(Err(ClipboardOperationError::NotConnected));
        return;
    };
    if !clipboard_capability_negotiated(
        connection.status.borrow().negotiated_capability_ids(),
        ClipboardKind::Write,
    ) {
        let _ = reply.send(Err(ClipboardOperationError::NotNegotiated));
        return;
    }
    let request_id = match RequestId::generate() {
        Ok(request_id) => request_id,
        Err(_) => {
            let _ = reply.send(Err(ClipboardOperationError::Random));
            return;
        }
    };
    let request = match clipboard_write_request(request_id, text) {
        Ok(request) => request,
        Err(error) => {
            let _ = reply.send(Err(error));
            return;
        }
    };
    if connection.actor.send_request(request).await.is_err() {
        let _ = reply.send(Err(ClipboardOperationError::Transport));
        return;
    }
    clipboard.outgoing.insert(
        request_id,
        PendingClipboard::Write {
            deadline: Instant::now() + CLIPBOARD_OPERATION_TIMEOUT,
            reply,
        },
    );
}

async fn start_clipboard_read(
    connected: Option<&ConnectedRuntime>,
    clipboard: &mut ClipboardRuntimeState,
    reply: oneshot::Sender<Result<String, ClipboardOperationError>>,
) {
    if clipboard.outgoing.len() >= RUNTIME_CAPACITY {
        let _ = reply.send(Err(ClipboardOperationError::ResourceLimit));
        return;
    }
    let Some(connection) = connected.filter(|connection| !connection.reconnecting) else {
        let _ = reply.send(Err(ClipboardOperationError::NotConnected));
        return;
    };
    if !clipboard_capability_negotiated(
        connection.status.borrow().negotiated_capability_ids(),
        ClipboardKind::Read,
    ) {
        let _ = reply.send(Err(ClipboardOperationError::NotNegotiated));
        return;
    }
    let request_id = match RequestId::generate() {
        Ok(request_id) => request_id,
        Err(_) => {
            let _ = reply.send(Err(ClipboardOperationError::Random));
            return;
        }
    };
    if connection
        .actor
        .send_request(clipboard_read_request(request_id))
        .await
        .is_err()
    {
        let _ = reply.send(Err(ClipboardOperationError::Transport));
        return;
    }
    clipboard.outgoing.insert(
        request_id,
        PendingClipboard::Read {
            deadline: Instant::now() + CLIPBOARD_OPERATION_TIMEOUT,
            reply,
        },
    );
}

async fn complete_clipboard_read(
    connected: Option<&ConnectedRuntime>,
    clipboard: &mut ClipboardRuntimeState,
    request_id: RequestId,
    result: Result<String, ClipboardPlatformError>,
) -> Result<(), ClipboardOperationError> {
    match clipboard.inbound.remove(&request_id) {
        Some(ClipboardKind::Read) => {}
        Some(ClipboardKind::Write) | None => return Err(ClipboardOperationError::Cancelled),
    }
    let connection = connected
        .filter(|connection| !connection.reconnecting)
        .ok_or(ClipboardOperationError::Cancelled)?;
    connection
        .actor
        .send_response(request_id, clipboard_read_completion(result))
        .await
        .map_err(|_| ClipboardOperationError::Transport)
}

async fn complete_clipboard_write(
    connected: Option<&ConnectedRuntime>,
    clipboard: &mut ClipboardRuntimeState,
    request_id: RequestId,
    result: Result<(), ClipboardPlatformError>,
) -> Result<(), ClipboardOperationError> {
    match clipboard.inbound.remove(&request_id) {
        Some(ClipboardKind::Write) => {}
        Some(ClipboardKind::Read) | None => return Err(ClipboardOperationError::Cancelled),
    }
    let connection = connected
        .filter(|connection| !connection.reconnecting)
        .ok_or(ClipboardOperationError::Cancelled)?;
    connection
        .actor
        .send_response(request_id, clipboard_write_completion(result))
        .await
        .map_err(|_| ClipboardOperationError::Transport)
}

async fn start_file_transfer_offer(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    offer: FileTransferOffer,
    reply: oneshot::Sender<Result<FileTransferAcceptance, FileTransferOperationError>>,
) {
    if file_transfer.outgoing.len() >= RUNTIME_CAPACITY {
        let _ = reply.send(Err(FileTransferOperationError::ResourceLimit));
        return;
    }
    if file_transfer
        .outgoing
        .values()
        .any(|pending| pending.offer.transfer_id() == offer.transfer_id())
        || matches!(
            file_transfer.source.get(&offer.transfer_id()),
            Some(SourceFileTransfer::Streaming { .. } | SourceFileTransfer::AwaitingResult { .. })
        )
    {
        let _ = reply.send(Err(FileTransferOperationError::AlreadyActive));
        return;
    }
    let Some(connection) = connected.filter(|connection| !connection.reconnecting) else {
        let _ = reply.send(Err(FileTransferOperationError::NotConnected));
        return;
    };
    if !file_transfer_capability_negotiated(connection.status.borrow().negotiated_capability_ids())
    {
        let _ = reply.send(Err(FileTransferOperationError::NotNegotiated));
        return;
    }
    if connection
        .actor
        .subscribe_event(file_transfer_result_subscription())
        .await
        .is_err()
    {
        let _ = reply.send(Err(FileTransferOperationError::Transport));
        return;
    }

    let request_id = match RequestId::generate() {
        Ok(request_id) => request_id,
        Err(_) => {
            let _ = reply.send(Err(FileTransferOperationError::Random));
            return;
        }
    };
    let request = match file_transfer_offer_request(request_id, &offer) {
        Ok(request) => request,
        Err(error) => {
            let _ = reply.send(Err(error));
            return;
        }
    };
    if connection.actor.send_request(request).await.is_err() {
        let _ = reply.send(Err(FileTransferOperationError::Transport));
        return;
    }

    if matches!(
        file_transfer.source.get(&offer.transfer_id()),
        Some(SourceFileTransfer::Ready { .. })
    ) {
        file_transfer.source.remove(&offer.transfer_id());
    }
    let _ = file_transfer.take_cached_result(offer.transfer_id());

    file_transfer.outgoing.insert(
        request_id,
        PendingFileOffer {
            offer,
            deadline: Instant::now() + FILE_TRANSFER_OFFER_TIMEOUT,
            reply,
        },
    );
}

async fn cancel_file_transfer_offer(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    transfer_id: TransferId,
) -> Result<(), FileTransferOperationError> {
    if let Some(request_id) = file_transfer
        .outgoing
        .iter()
        .find_map(|(request_id, pending)| {
            (pending.offer.transfer_id() == transfer_id).then_some(*request_id)
        })
    {
        let Some(pending) = file_transfer.outgoing.remove(&request_id) else {
            return Ok(());
        };
        let result =
            if let Some(connection) = connected.filter(|connection| !connection.reconnecting) {
                connection
                    .actor
                    .send_cancel(request_id)
                    .await
                    .map_err(|_| FileTransferOperationError::Transport)
            } else {
                Ok(())
            };
        let _ = pending
            .reply
            .send(Err(FileTransferOperationError::Cancelled));
        return result;
    }

    let Some(source) = file_transfer.source.remove(&transfer_id) else {
        return Ok(());
    };
    let Some(connection) = connected.filter(|connection| !connection.reconnecting) else {
        return Ok(());
    };

    match source {
        SourceFileTransfer::Ready { operation_id, .. } => {
            let Some(session_id) = connection.status.borrow().session_id() else {
                return Ok(());
            };
            let stream_id = StreamId::generate().map_err(|_| FileTransferOperationError::Random)?;
            connection
                .actor
                .open_data_stream(file_transfer_source_stream_open(
                    session_id,
                    stream_id,
                    operation_id,
                ))
                .await
                .map_err(|_| FileTransferOperationError::Transport)?;
            connection
                .actor
                .cancel_outbound_stream(stream_id)
                .await
                .map_err(|_| FileTransferOperationError::Transport)
        }
        SourceFileTransfer::Streaming { stream_id, .. } => connection
            .actor
            .cancel_outbound_stream(stream_id)
            .await
            .map_err(|_| FileTransferOperationError::Transport),
        source @ SourceFileTransfer::AwaitingResult { .. } => {
            file_transfer.source.insert(transfer_id, source);
            Err(FileTransferOperationError::AlreadyActive)
        }
    }
}

async fn decline_file_transfer_request(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    request_id: RequestId,
) -> Result<(), FileTransferOperationError> {
    if file_transfer.inbound.remove(&request_id).is_none() {
        return Err(FileTransferOperationError::Cancelled);
    }
    let connection = connected
        .filter(|connection| !connection.reconnecting)
        .ok_or(FileTransferOperationError::Cancelled)?;
    connection
        .actor
        .send_response(request_id, file_transfer_cancelled_failure())
        .await
        .map_err(|_| FileTransferOperationError::Transport)
}

async fn complete_file_transfer_ready(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    request_id: RequestId,
    resume_offset: u64,
) -> Result<(), FileTransferOperationError> {
    let request = file_transfer
        .inbound
        .get(&request_id)
        .cloned()
        .ok_or(FileTransferOperationError::Cancelled)?;
    request
        .offer()
        .validate_resume_offset(resume_offset)
        .map_err(|_| FileTransferOperationError::InvalidResumeOffset)?;
    let connection = connected
        .filter(|connection| !connection.reconnecting)
        .ok_or(FileTransferOperationError::Cancelled)?;

    let operation_id = connection
        .actor
        .issue_stream_operation(
            CapabilityId::parse(FILE_TRANSFER_CAPABILITY_ID)
                .expect("file transfer capability id is canonical"),
            CapabilityVersion::new(2, 0),
            OperationName::parse("receive").expect("file transfer receive operation is canonical"),
            FILE_TRANSFER_OPERATION_LIFETIME,
            UsePolicy::SingleStream,
        )
        .await
        .map_err(|_| FileTransferOperationError::Cancelled)?;

    let response = match file_transfer_ready_response(&request, resume_offset, operation_id) {
        Ok(response) => response,
        Err(error) => {
            let _ = connection.actor.cancel_stream_operation(operation_id).await;
            return Err(error);
        }
    };
    if connection
        .actor
        .send_response(request_id, response)
        .await
        .is_err()
    {
        let _ = connection.actor.cancel_stream_operation(operation_id).await;
        return Err(FileTransferOperationError::Transport);
    }

    file_transfer.inbound.remove(&request_id);
    file_transfer
        .destination_ready
        .push(DestinationReadyFileTransfer {
            transfer_id: request.offer().transfer_id(),
            operation_id,
            resume_offset,
            deadline: Instant::now() + FILE_TRANSFER_OPERATION_LIFETIME,
        });
    Ok(())
}

async fn complete_file_transfer_already_complete(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    request_id: RequestId,
) -> Result<(), FileTransferOperationError> {
    let request = file_transfer
        .inbound
        .get(&request_id)
        .cloned()
        .ok_or(FileTransferOperationError::Cancelled)?;
    let connection = connected
        .filter(|connection| !connection.reconnecting)
        .ok_or(FileTransferOperationError::Cancelled)?;
    let response = file_transfer_already_complete_response(&request)?;
    connection
        .actor
        .send_response(request_id, response)
        .await
        .map_err(|_| FileTransferOperationError::Transport)?;
    file_transfer.inbound.remove(&request_id);
    Ok(())
}

async fn open_file_transfer_stream(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    transfer_id: TransferId,
) -> Result<FileTransferSourceStream, FileTransferOperationError> {
    if let Some(result) = file_transfer.take_cached_result(transfer_id) {
        return match result.outcome() {
            FileTransferTerminalOutcome::Cancelled => Err(FileTransferOperationError::Remote(
                ProtocolErrorCode::Cancelled,
            )),
            FileTransferTerminalOutcome::Completed
            | FileTransferTerminalOutcome::IntegrityFailed
            | FileTransferTerminalOutcome::StorageFailed => {
                Err(FileTransferOperationError::InvalidResponse)
            }
        };
    }

    let (operation_id, resume_offset, deadline) = match file_transfer.source.get(&transfer_id) {
        Some(SourceFileTransfer::Ready {
            operation_id,
            resume_offset,
            deadline,
        }) => (*operation_id, *resume_offset, *deadline),
        Some(_) => return Err(FileTransferOperationError::AlreadyActive),
        None => return Err(FileTransferOperationError::InvalidStream),
    };
    if deadline <= Instant::now() {
        file_transfer.source.remove(&transfer_id);
        return Err(FileTransferOperationError::TimedOut);
    }

    let connection = connected
        .filter(|connection| !connection.reconnecting)
        .ok_or(FileTransferOperationError::NotConnected)?;
    let session_id = connection
        .status
        .borrow()
        .session_id()
        .ok_or(FileTransferOperationError::NotConnected)?;
    let stream_id = StreamId::generate().map_err(|_| FileTransferOperationError::Random)?;
    let open = file_transfer_source_stream_open(session_id, stream_id, operation_id);
    connection
        .actor
        .open_data_stream(open)
        .await
        .map_err(|_| FileTransferOperationError::Transport)?;

    file_transfer.source.insert(
        transfer_id,
        SourceFileTransfer::Streaming {
            stream_id,
            resume_offset,
        },
    );
    Ok(FileTransferSourceStream::new(
        transfer_id,
        stream_id,
        resume_offset,
    ))
}

async fn send_file_transfer_chunk(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &FileTransferRuntimeState,
    stream: FileTransferSourceStream,
    chunk: Vec<u8>,
) -> Result<(), FileTransferChunkError> {
    let active = matches!(
        file_transfer.source.get(&stream.transfer_id()),
        Some(SourceFileTransfer::Streaming {
            stream_id,
            resume_offset,
        }) if *stream_id == stream.stream_id() && *resume_offset == stream.resume_offset()
    );
    if !active {
        return Err(FileTransferChunkError::Closed(Some(chunk)));
    }
    let Some(connection) = connected.filter(|connection| !connection.reconnecting) else {
        return Err(FileTransferChunkError::Closed(Some(chunk)));
    };
    connection
        .actor
        .send_stream_chunk(stream.stream_id(), chunk)
        .await
        .map_err(map_file_transfer_chunk_error)
}

async fn finish_file_transfer_stream(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    stream: FileTransferSourceStream,
    cancel: bool,
    reply: oneshot::Sender<Result<FileTransferResult, FileTransferOperationError>>,
) {
    if let Some(result) = file_transfer.take_cached_result(stream.transfer_id()) {
        let _ = reply.send(Ok(result));
        return;
    }

    let active = matches!(
        file_transfer.source.get(&stream.transfer_id()),
        Some(SourceFileTransfer::Streaming {
            stream_id,
            resume_offset,
        }) if *stream_id == stream.stream_id() && *resume_offset == stream.resume_offset()
    );
    if !active {
        let _ = reply.send(Err(FileTransferOperationError::InvalidStream));
        return;
    }
    let Some(connection) = connected.filter(|connection| !connection.reconnecting) else {
        let _ = reply.send(Err(FileTransferOperationError::NotConnected));
        return;
    };

    let operation = if cancel {
        connection
            .actor
            .cancel_outbound_stream(stream.stream_id())
            .await
    } else {
        connection
            .actor
            .finish_data_stream(stream.stream_id())
            .await
    };
    if operation.is_err() {
        file_transfer.source.remove(&stream.transfer_id());
        let _ = reply.send(Err(FileTransferOperationError::Transport));
        return;
    }

    file_transfer.source.insert(
        stream.transfer_id(),
        SourceFileTransfer::AwaitingResult {
            deadline: Instant::now() + FILE_TRANSFER_OFFER_TIMEOUT,
            reply,
        },
    );
}

async fn recover_closed_file_transfer_result(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    stream: FileTransferSourceStream,
    reply: oneshot::Sender<Result<FileTransferResult, FileTransferOperationError>>,
) {
    if let Some(result) = file_transfer.take_cached_result(stream.transfer_id()) {
        file_transfer.source.remove(&stream.transfer_id());
        let _ = reply.send(Ok(result));
        return;
    }

    let active = matches!(
        file_transfer.source.get(&stream.transfer_id()),
        Some(SourceFileTransfer::Streaming {
            stream_id,
            resume_offset,
        }) if *stream_id == stream.stream_id() && *resume_offset == stream.resume_offset()
    );
    if !active {
        let _ = reply.send(Err(FileTransferOperationError::InvalidStream));
        return;
    }
    let Some(connection) = connected.filter(|connection| !connection.reconnecting) else {
        file_transfer.source.remove(&stream.transfer_id());
        let _ = reply.send(Err(FileTransferOperationError::NotConnected));
        return;
    };

    let _ = connection
        .actor
        .cancel_outbound_stream(stream.stream_id())
        .await;
    file_transfer.source.insert(
        stream.transfer_id(),
        SourceFileTransfer::AwaitingResult {
            deadline: Instant::now() + FILE_TRANSFER_OFFER_TIMEOUT,
            reply,
        },
    );
}

async fn cancel_file_transfer_receive(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    transfer_id: TransferId,
) -> Result<Option<StreamId>, FileTransferOperationError> {
    let connection = connected
        .filter(|connection| !connection.reconnecting)
        .ok_or(FileTransferOperationError::NotConnected)?;

    if let Some(position) = file_transfer
        .destination_ready
        .iter()
        .position(|ready| ready.transfer_id == transfer_id)
    {
        let ready = file_transfer.destination_ready.remove(position);
        let _ = connection
            .actor
            .cancel_stream_operation(ready.operation_id)
            .await;
        let _ = send_file_transfer_terminal(
            connection,
            transfer_id,
            FileTransferTerminalOutcome::Cancelled,
        )
        .await;
        return Ok(None);
    }

    if let Some(stream_id) = file_transfer
        .inbound_streams
        .iter()
        .find_map(|(stream_id, active)| (active.transfer_id == transfer_id).then_some(*stream_id))
    {
        let _ = connection.actor.cancel_inbound_stream(stream_id).await;
        file_transfer.inbound_streams.remove(&stream_id);
        let _ = send_file_transfer_terminal(
            connection,
            transfer_id,
            FileTransferTerminalOutcome::Cancelled,
        )
        .await;
        return Ok(Some(stream_id));
    }

    if let Some(position) = file_transfer
        .terminal_ready
        .iter()
        .position(|terminal| terminal.transfer_id == transfer_id)
    {
        let terminal = file_transfer.terminal_ready.remove(position);
        let _ = send_file_transfer_terminal(
            connection,
            transfer_id,
            FileTransferTerminalOutcome::Cancelled,
        )
        .await;
        return Ok(Some(terminal.stream_id));
    }

    Err(FileTransferOperationError::InvalidStream)
}

async fn fail_file_transfer_receive(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    stream_id: StreamId,
    transfer_id: TransferId,
    outcome: FileTransferTerminalOutcome,
) -> Result<(), FileTransferOperationError> {
    if !matches!(
        outcome,
        FileTransferTerminalOutcome::IntegrityFailed | FileTransferTerminalOutcome::StorageFailed
    ) {
        return Err(FileTransferOperationError::InvalidResponse);
    }

    let active = file_transfer
        .inbound_streams
        .get(&stream_id)
        .copied()
        .ok_or(FileTransferOperationError::InvalidStream)?;
    if active.transfer_id != transfer_id {
        return Err(FileTransferOperationError::InvalidStream);
    }

    let connection = connected
        .filter(|connection| !connection.reconnecting)
        .ok_or(FileTransferOperationError::NotConnected)?;
    let _ = connection.actor.cancel_inbound_stream(stream_id).await;
    file_transfer.inbound_streams.remove(&stream_id);
    send_file_transfer_terminal(connection, transfer_id, outcome).await
}

async fn complete_file_transfer_terminal(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    transfer_id: TransferId,
    outcome: FileTransferTerminalOutcome,
) -> Result<(), FileTransferOperationError> {
    let position = file_transfer
        .terminal_ready
        .iter()
        .position(|terminal| terminal.transfer_id == transfer_id)
        .ok_or(FileTransferOperationError::InvalidStream)?;
    let connection = connected
        .filter(|connection| !connection.reconnecting)
        .ok_or(FileTransferOperationError::NotConnected)?;
    send_file_transfer_terminal(connection, transfer_id, outcome).await?;
    file_transfer.terminal_ready.remove(position);
    Ok(())
}

async fn send_file_transfer_terminal(
    connection: &ConnectedRuntime,
    transfer_id: TransferId,
    outcome: FileTransferTerminalOutcome,
) -> Result<(), FileTransferOperationError> {
    let event =
        file_transfer_terminal_result_event(file_transfer_terminal_result(transfer_id, outcome))?;
    connection
        .actor
        .send_event(event)
        .await
        .map_err(|_| FileTransferOperationError::Transport)
}

fn remember_terminal_ready(
    file_transfer: &mut FileTransferRuntimeState,
    transfer_id: TransferId,
    stream_id: StreamId,
) {
    if file_transfer
        .terminal_ready
        .iter()
        .any(|terminal| terminal.transfer_id == transfer_id)
    {
        return;
    }
    if file_transfer.terminal_ready.len() >= RUNTIME_CAPACITY {
        file_transfer.terminal_ready.remove(0);
    }
    file_transfer
        .terminal_ready
        .push(TerminalReadyFileTransfer {
            transfer_id,
            stream_id,
            deadline: Instant::now() + FILE_TRANSFER_OFFER_TIMEOUT,
        });
}

async fn fail_inbound_file_transfer(
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
    stream_id: StreamId,
    transfer_id: TransferId,
    outcome: FileTransferTerminalOutcome,
) {
    file_transfer.inbound_streams.remove(&stream_id);
    file_transfer.notify_transfer_interrupted(transfer_id);
    if let Some(connection) = connected.filter(|connection| !connection.reconnecting) {
        let _ = connection.actor.cancel_inbound_stream(stream_id).await;
        let _ = send_file_transfer_terminal(connection, transfer_id, outcome).await;
    }
}

async fn handle_file_transfer_result_event(
    event: &crosslab_protocol::Event,
    connected: Option<&ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
) {
    let result = match decode_file_transfer_terminal_result(event) {
        Ok(Some(result)) => result,
        Ok(None) => return,
        Err(_) => {
            file_transfer.cancel_all(FileTransferOperationError::InvalidResponse);
            return;
        }
    };
    let transfer_id = result.transfer_id();

    match file_transfer.source.remove(&transfer_id) {
        Some(SourceFileTransfer::AwaitingResult { reply, .. }) => {
            let _ = reply.send(Ok(result));
        }
        Some(SourceFileTransfer::Streaming { stream_id, .. })
            if result.outcome() != FileTransferTerminalOutcome::Completed =>
        {
            if let Some(connection) = connected.filter(|connection| !connection.reconnecting) {
                let _ = connection.actor.cancel_outbound_stream(stream_id).await;
            }
            file_transfer.cache_result(result);
        }
        Some(source @ SourceFileTransfer::Ready { .. })
            if result.outcome() == FileTransferTerminalOutcome::Completed =>
        {
            file_transfer.source.insert(transfer_id, source);
        }
        Some(SourceFileTransfer::Ready { .. }) => {
            file_transfer.cache_result(result);
        }
        Some(source @ SourceFileTransfer::Streaming { .. }) => {
            file_transfer.source.insert(transfer_id, source);
        }
        None => {}
    }
}

async fn expire_file_transfer_offers(
    connected: &mut Option<ConnectedRuntime>,
    file_transfer: &mut FileTransferRuntimeState,
) {
    let now = Instant::now();
    let expired = file_transfer
        .outgoing
        .iter()
        .filter_map(|(request_id, pending)| (pending.deadline <= now).then_some(*request_id))
        .collect::<Vec<_>>();

    for request_id in expired {
        let Some(pending) = file_transfer.outgoing.remove(&request_id) else {
            continue;
        };
        if let Some(connection) = connected
            .as_ref()
            .filter(|connection| !connection.reconnecting)
        {
            let _ = connection.actor.send_cancel(request_id).await;
        }
        let _ = pending
            .reply
            .send(Err(FileTransferOperationError::TimedOut));
    }

    let expired_operations = file_transfer
        .destination_ready
        .iter()
        .filter_map(|ready| {
            (ready.deadline <= now).then_some((ready.operation_id, ready.transfer_id))
        })
        .collect::<Vec<_>>();
    file_transfer
        .destination_ready
        .retain(|ready| ready.deadline > now);
    for (operation_id, transfer_id) in expired_operations {
        if let Some(connection) = connected
            .as_ref()
            .filter(|connection| !connection.reconnecting)
        {
            let _ = connection.actor.cancel_stream_operation(operation_id).await;
        }
        file_transfer.notify_transfer_interrupted(transfer_id);
    }

    let expired_source = file_transfer
        .source
        .iter()
        .filter_map(|(transfer_id, source)| match source {
            SourceFileTransfer::Ready { deadline, .. }
            | SourceFileTransfer::AwaitingResult { deadline, .. }
                if *deadline <= now =>
            {
                Some(*transfer_id)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    for transfer_id in expired_source {
        if let Some(SourceFileTransfer::AwaitingResult { reply, .. }) =
            file_transfer.source.remove(&transfer_id)
        {
            let _ = reply.send(Err(FileTransferOperationError::TimedOut));
        }
    }

    file_transfer
        .terminal_ready
        .retain(|terminal| terminal.deadline > now);
}

async fn handle_runtime_event(
    event: NodeEvent,
    connected: &mut Option<ConnectedRuntime>,
    clipboard: &mut ClipboardRuntimeState,
    file_transfer: &mut FileTransferRuntimeState,
) {
    match event {
        NodeEvent::RequestDispatched(request) => {
            let request_id = request.request_id();
            let result = if request.capability_id().as_str() == FILE_TRANSFER_CAPABILITY_ID {
                let Some(source_device_id) = connected
                    .as_ref()
                    .filter(|connection| !connection.reconnecting)
                    .map(|connection| connection.peer_id)
                else {
                    return;
                };
                match decode_file_transfer(&request, source_device_id) {
                    Ok(platform_request) => {
                        if file_transfer.destination_capacity_used() >= RUNTIME_CAPACITY {
                            Some(file_transfer_resource_failure())
                        } else {
                            match file_transfer.requests_tx.try_send(platform_request.clone()) {
                                Ok(()) => {
                                    file_transfer.inbound.insert(request_id, platform_request);
                                    None
                                }
                                Err(mpsc::error::TrySendError::Full(_)) => {
                                    Some(file_transfer_resource_failure())
                                }
                                Err(mpsc::error::TrySendError::Closed(_)) => {
                                    Some(file_transfer_internal_failure())
                                }
                            }
                        }
                    }
                    Err(result) => Some(result),
                }
            } else {
                match decode_clipboard(&request) {
                    Ok(platform_request) => {
                        if clipboard.inbound.len() >= RUNTIME_CAPACITY {
                            Some(clipboard_resource_failure())
                        } else {
                            let kind = match &platform_request {
                                ClipboardRequest::Read { .. } => ClipboardKind::Read,
                                ClipboardRequest::Write { .. } => ClipboardKind::Write,
                            };
                            match clipboard.requests_tx.try_send(platform_request) {
                                Ok(()) => {
                                    clipboard.inbound.insert(request_id, kind);
                                    None
                                }
                                Err(mpsc::error::TrySendError::Full(_)) => {
                                    Some(clipboard_resource_failure())
                                }
                                Err(mpsc::error::TrySendError::Closed(_)) => {
                                    Some(clipboard_internal_failure())
                                }
                            }
                        }
                    }
                    Err(result) => Some(result),
                }
            };

            if let Some(result) = result
                && let Some(connection) = connected
                    .as_ref()
                    .filter(|connection| !connection.reconnecting)
            {
                let _ = connection.actor.send_response(request_id, result).await;
            }
        }
        NodeEvent::Response(response) => {
            if let Some(pending) = clipboard.outgoing.remove(&response.request_id()) {
                let result = decode_clipboard_response(pending.kind(), response.result());
                pending.finish(result);
            } else if let Some(pending) = file_transfer.outgoing.remove(&response.request_id()) {
                let mut result = decode_file_transfer_response(&pending.offer, response.result());
                if let Ok(FileTransferAcceptance::Ready {
                    transfer_id,
                    resume_offset,
                    operation_id,
                }) = result.as_ref()
                {
                    if file_transfer.source.contains_key(transfer_id)
                        || file_transfer.source.len() < RUNTIME_CAPACITY
                    {
                        file_transfer.source.insert(
                            *transfer_id,
                            SourceFileTransfer::Ready {
                                operation_id: *operation_id,
                                resume_offset: *resume_offset,
                                deadline: Instant::now() + FILE_TRANSFER_OPERATION_LIFETIME,
                            },
                        );
                    } else {
                        result = Err(FileTransferOperationError::ResourceLimit);
                    }
                }
                let _ = pending.reply.send(result);
            }
        }
        NodeEvent::RequestCancelled(request_id) => {
            clipboard.inbound.remove(&request_id);
            if let Some(request) = file_transfer.inbound.remove(&request_id) {
                file_transfer.notify_request_cancelled(request_id, request.offer().transfer_id());
            }
        }
        NodeEvent::SessionClosed(_) => {
            clipboard.cancel_all(ClipboardOperationError::Cancelled);
            file_transfer.cancel_all(FileTransferOperationError::Cancelled);
        }
        NodeEvent::Event(event) => {
            handle_file_transfer_result_event(&event, connected.as_ref(), file_transfer).await;
        }
        NodeEvent::Stream(RuntimeStreamEvent::Opened(stream)) => {
            let Some(position) = file_transfer
                .destination_ready
                .iter()
                .position(|ready| ready.operation_id == stream.operation_id())
            else {
                return;
            };
            let ready = file_transfer.destination_ready.remove(position);
            let stream_id = stream.stream_id();
            file_transfer.inbound_streams.insert(
                stream_id,
                ActiveInboundFileTransfer {
                    transfer_id: ready.transfer_id,
                },
            );
            let event = FileTransferDataEvent::Opened {
                transfer_id: ready.transfer_id,
                stream_id,
                resume_offset: ready.resume_offset,
            };
            if file_transfer.data_tx.try_send(event).is_err() {
                fail_inbound_file_transfer(
                    connected.as_ref(),
                    file_transfer,
                    stream_id,
                    ready.transfer_id,
                    FileTransferTerminalOutcome::StorageFailed,
                )
                .await;
            }
        }
        NodeEvent::Stream(RuntimeStreamEvent::Chunk(chunk)) => {
            let stream_id = chunk.stream_id();
            let Some(active) = file_transfer.inbound_streams.get(&stream_id).copied() else {
                return;
            };
            let event = FileTransferDataEvent::Chunk(FileTransferDataChunk::new(
                active.transfer_id,
                stream_id,
                chunk.into_bytes(),
            ));
            if file_transfer.data_tx.try_send(event).is_err() {
                fail_inbound_file_transfer(
                    connected.as_ref(),
                    file_transfer,
                    stream_id,
                    active.transfer_id,
                    FileTransferTerminalOutcome::StorageFailed,
                )
                .await;
            }
        }
        NodeEvent::Stream(RuntimeStreamEvent::Finished(stream_id)) => {
            let Some(active) = file_transfer.inbound_streams.remove(&stream_id) else {
                return;
            };
            remember_terminal_ready(file_transfer, active.transfer_id, stream_id);
            let event = FileTransferDataEvent::Finished {
                transfer_id: active.transfer_id,
                stream_id,
            };
            if file_transfer.data_tx.try_send(event).is_err() {
                file_transfer
                    .terminal_ready
                    .retain(|terminal| terminal.transfer_id != active.transfer_id);
                file_transfer.notify_transfer_interrupted(active.transfer_id);
                if let Some(connection) = connected
                    .as_ref()
                    .filter(|connection| !connection.reconnecting)
                {
                    let _ = send_file_transfer_terminal(
                        connection,
                        active.transfer_id,
                        FileTransferTerminalOutcome::StorageFailed,
                    )
                    .await;
                }
            }
        }
        NodeEvent::Stream(RuntimeStreamEvent::Cancelled(stream_id)) => {
            let Some(active) = file_transfer.inbound_streams.remove(&stream_id) else {
                return;
            };
            remember_terminal_ready(file_transfer, active.transfer_id, stream_id);
            let event = FileTransferDataEvent::Cancelled {
                transfer_id: active.transfer_id,
                stream_id,
            };
            if file_transfer.data_tx.try_send(event).is_err() {
                file_transfer
                    .terminal_ready
                    .retain(|terminal| terminal.transfer_id != active.transfer_id);
                file_transfer.notify_transfer_interrupted(active.transfer_id);
                if let Some(connection) = connected
                    .as_ref()
                    .filter(|connection| !connection.reconnecting)
                {
                    let _ = send_file_transfer_terminal(
                        connection,
                        active.transfer_id,
                        FileTransferTerminalOutcome::Cancelled,
                    )
                    .await;
                }
            }
        }
        NodeEvent::CapabilitiesUpdated | NodeEvent::ProtocolFailure(_) => {}
    }
}

async fn expire_clipboard_operations(
    connected: &mut Option<ConnectedRuntime>,
    clipboard: &mut ClipboardRuntimeState,
) {
    let now = Instant::now();
    let expired = clipboard
        .outgoing
        .iter()
        .filter_map(|(request_id, pending)| (pending.deadline() <= now).then_some(*request_id))
        .collect::<Vec<_>>();

    for request_id in expired {
        let Some(pending) = clipboard.outgoing.remove(&request_id) else {
            continue;
        };
        if let Some(connection) = connected
            .as_ref()
            .filter(|connection| !connection.reconnecting)
        {
            let _ = connection.actor.send_cancel(request_id).await;
        }
        pending.cancel(ClipboardOperationError::TimedOut);
    }
}

async fn apply_policy_update(
    policy_rx: &mut watch::Receiver<PolicyState>,
    policy: &mut PolicyState,
    connected: &mut Option<ConnectedRuntime>,
    permissions_tx: &watch::Sender<PermissionSnapshot>,
    status_tx: &watch::Sender<PresenceSnapshot>,
    clipboard: &mut ClipboardRuntimeState,
    file_transfer: &mut FileTransferRuntimeState,
) {
    let next = policy_rx.borrow_and_update().clone();
    if *policy == next || next.revision() <= policy.revision() {
        return;
    }

    clipboard.cancel_all(ClipboardOperationError::Cancelled);
    file_transfer.cancel_all(FileTransferOperationError::Cancelled);
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
    clipboard_availability: ClipboardAvailability,
    file_transfer_availability: FileTransferAvailability,
) -> Option<(RuntimeActorSession, watch::Receiver<bool>)> {
    let (session, transport) = session.into_parts();
    let closed = transport.subscribe_closed();
    let control_ready = transport.subscribe_control_ready();
    let stream_ready = transport.subscribe_stream_ready();
    let node = RuntimeNode::new_owned(
        session,
        Arc::new(transport),
        policy.clone(),
        runtime_local_capabilities(clipboard_availability, file_transfer_availability),
        NetworkClass::Local,
        NonZeroUsize::new(RUNTIME_CAPACITY)?,
    )
    .ok()?;
    Some((
        RuntimeActorSession::new(node, peer_trust)
            .with_control_ready(control_ready)
            .with_stream_ready(stream_ready),
        closed,
    ))
}

fn runtime_local_capabilities(
    clipboard_availability: ClipboardAvailability,
    file_transfer_availability: FileTransferAvailability,
) -> Vec<crosslab_policy::LocalCapability> {
    let mut capabilities = clipboard_local_capabilities(clipboard_availability);
    capabilities.extend(file_transfer_local_capabilities(file_transfer_availability));
    capabilities
}

fn runtime_advertisement(
    clipboard_availability: ClipboardAvailability,
    file_transfer_availability: FileTransferAvailability,
) -> CapabilityAdvertisement {
    let clipboard = clipboard_advertisement(clipboard_availability);
    let files = file_transfer_advertisement(file_transfer_availability);
    let mut entries = Vec::with_capacity(clipboard.len() + files.len());
    entries.extend_from_slice(clipboard.entries());
    entries.extend_from_slice(files.entries());
    CapabilityAdvertisement::new(entries).expect("product capability advertisement is bounded")
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
