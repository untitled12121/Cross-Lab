use core::fmt;
use std::sync::{Arc, Mutex};

use crosslab_agent::{
    FileTransferChunkError, FileTransferCompletionTombstone, FileTransferDataEvent,
    FileTransferHasher, FileTransferIdentity, FileTransferIntegrityError, FileTransferLocalLocator,
    FileTransferOperationError, FileTransferPartialState, FileTransferRecoveryAction,
    FileTransferRequest, FileTransferSourceStream, FileTransferStateError, FileTransferStateMatch,
    FileTransferStateSnapshot, FileTransferVerifier,
};
use crosslab_identity::DeviceId;
use crosslab_protocol::{
    FileTransferAcceptance, FileTransferOffer, FileTransferProfileError, FileTransferResult,
    FileTransferTerminalOutcome, ProtocolErrorCode, RequestId, StreamId, TransferId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferAcceptanceKind {
    Ready,
    AlreadyComplete,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileFileTransferAcceptance {
    pub kind: MobileFileTransferAcceptanceKind,
    pub transfer_id: Vec<u8>,
    pub resume_offset: Option<u64>,
}

impl MobileFileTransferAcceptance {
    pub(crate) fn from_protocol(acceptance: FileTransferAcceptance) -> Self {
        match acceptance {
            FileTransferAcceptance::Ready {
                transfer_id,
                resume_offset,
                ..
            } => Self {
                kind: MobileFileTransferAcceptanceKind::Ready,
                transfer_id: transfer_id.to_bytes().to_vec(),
                resume_offset: Some(resume_offset),
            },
            FileTransferAcceptance::AlreadyComplete { transfer_id } => Self {
                kind: MobileFileTransferAcceptanceKind::AlreadyComplete,
                transfer_id: transfer_id.to_bytes().to_vec(),
                resume_offset: None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferTerminalOutcome {
    Completed,
    Cancelled,
    IntegrityFailed,
    StorageFailed,
}

impl From<FileTransferTerminalOutcome> for MobileFileTransferTerminalOutcome {
    fn from(outcome: FileTransferTerminalOutcome) -> Self {
        match outcome {
            FileTransferTerminalOutcome::Completed => Self::Completed,
            FileTransferTerminalOutcome::Cancelled => Self::Cancelled,
            FileTransferTerminalOutcome::IntegrityFailed => Self::IntegrityFailed,
            FileTransferTerminalOutcome::StorageFailed => Self::StorageFailed,
        }
    }
}

impl From<MobileFileTransferTerminalOutcome> for FileTransferTerminalOutcome {
    fn from(outcome: MobileFileTransferTerminalOutcome) -> Self {
        match outcome {
            MobileFileTransferTerminalOutcome::Completed => Self::Completed,
            MobileFileTransferTerminalOutcome::Cancelled => Self::Cancelled,
            MobileFileTransferTerminalOutcome::IntegrityFailed => Self::IntegrityFailed,
            MobileFileTransferTerminalOutcome::StorageFailed => Self::StorageFailed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileFileTransferResult {
    pub transfer_id: Vec<u8>,
    pub outcome: MobileFileTransferTerminalOutcome,
}

impl MobileFileTransferResult {
    pub(crate) fn from_protocol(result: FileTransferResult) -> Self {
        Self {
            transfer_id: result.transfer_id().to_bytes().to_vec(),
            outcome: result.outcome().into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferCancellationKind {
    Request,
    Transfer,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileFileTransferCancellation {
    pub kind: MobileFileTransferCancellationKind,
    pub request_id: Option<Vec<u8>>,
    pub transfer_id: Vec<u8>,
}

impl MobileFileTransferCancellation {
    pub(crate) fn from_agent(cancellation: crosslab_agent::FileTransferCancellation) -> Self {
        match cancellation {
            crosslab_agent::FileTransferCancellation::Request {
                request_id,
                transfer_id,
            } => Self {
                kind: MobileFileTransferCancellationKind::Request,
                request_id: Some(request_id.to_bytes().to_vec()),
                transfer_id: transfer_id.to_bytes().to_vec(),
            },
            crosslab_agent::FileTransferCancellation::Transfer { transfer_id } => Self {
                kind: MobileFileTransferCancellationKind::Transfer,
                request_id: None,
                transfer_id: transfer_id.to_bytes().to_vec(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferDataKind {
    Opened,
    Chunk,
    Finished,
    Cancelled,
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferDataEvent {
    kind: MobileFileTransferDataKind,
    transfer_id: TransferId,
    stream_id: StreamId,
    resume_offset: Option<u64>,
    bytes: Mutex<Option<Vec<u8>>>,
}

impl MobileFileTransferDataEvent {
    pub(crate) fn from_agent(event: FileTransferDataEvent) -> Self {
        match event {
            FileTransferDataEvent::Opened {
                transfer_id,
                stream_id,
                resume_offset,
            } => Self {
                kind: MobileFileTransferDataKind::Opened,
                transfer_id,
                stream_id,
                resume_offset: Some(resume_offset),
                bytes: Mutex::new(None),
            },
            FileTransferDataEvent::Chunk(chunk) => Self {
                kind: MobileFileTransferDataKind::Chunk,
                transfer_id: chunk.transfer_id(),
                stream_id: chunk.stream_id(),
                resume_offset: None,
                bytes: Mutex::new(Some(chunk.into_bytes())),
            },
            FileTransferDataEvent::Finished {
                transfer_id,
                stream_id,
            } => Self {
                kind: MobileFileTransferDataKind::Finished,
                transfer_id,
                stream_id,
                resume_offset: None,
                bytes: Mutex::new(None),
            },
            FileTransferDataEvent::Cancelled {
                transfer_id,
                stream_id,
            } => Self {
                kind: MobileFileTransferDataKind::Cancelled,
                transfer_id,
                stream_id,
                resume_offset: None,
                bytes: Mutex::new(None),
            },
        }
    }
}

impl fmt::Debug for MobileFileTransferDataEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let byte_len = self
            .bytes
            .lock()
            .ok()
            .and_then(|bytes| bytes.as_ref().map(Vec::len));
        formatter
            .debug_struct("MobileFileTransferDataEvent")
            .field("kind", &self.kind)
            .field("transfer_id", &self.transfer_id)
            .field("stream_id", &self.stream_id)
            .field("resume_offset", &self.resume_offset)
            .field("byte_len", &byte_len)
            .finish()
    }
}

#[uniffi::export]
impl MobileFileTransferDataEvent {
    pub fn kind(&self) -> MobileFileTransferDataKind {
        self.kind
    }

    pub fn transfer_id(&self) -> Vec<u8> {
        self.transfer_id.to_bytes().to_vec()
    }

    pub fn stream_id(&self) -> Vec<u8> {
        self.stream_id.to_bytes().to_vec()
    }

    pub fn resume_offset(&self) -> Option<u64> {
        self.resume_offset
    }

    pub fn take_bytes(&self) -> Result<Option<Vec<u8>>, MobileFileTransferError> {
        self.bytes
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)
            .map(|mut bytes| bytes.take())
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferRequest {
    request_id: RequestId,
    source_device_id: DeviceId,
    offer: FileTransferOffer,
}

impl MobileFileTransferRequest {
    pub(crate) fn from_agent(request: FileTransferRequest) -> Self {
        Self {
            request_id: request.request_id(),
            source_device_id: request.source_device_id(),
            offer: request.into_offer(),
        }
    }

}

impl fmt::Debug for MobileFileTransferRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MobileFileTransferRequest")
            .field("request_id", &self.request_id)
            .field("source_device_id", &self.source_device_id)
            .field("offer", &self.offer)
            .finish()
    }
}

#[uniffi::export]
impl MobileFileTransferRequest {
    pub fn request_id(&self) -> Vec<u8> {
        self.request_id.to_bytes().to_vec()
    }

    pub fn source_device_id(&self) -> String {
        hex(self.source_device_id.as_bytes())
    }

    pub fn transfer_id(&self) -> Vec<u8> {
        self.offer.transfer_id().to_bytes().to_vec()
    }

    pub fn display_name(&self) -> String {
        self.offer.display_name().to_owned()
    }

    pub fn file_size(&self) -> u64 {
        self.offer.file_size()
    }

    pub fn verifier(&self) -> Arc<MobileFileTransferVerifier> {
        Arc::new(MobileFileTransferVerifier::new(&self.offer))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferRetainedKind {
    New,
    Partial,
    AlreadyComplete,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileFileTransferRetainedMatch {
    pub kind: MobileFileTransferRetainedKind,
    pub durable_offset: Option<u64>,
    pub local_locator: Option<Vec<u8>>,
    pub truncate_to: Option<u64>,
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferState {
    state: Mutex<FileTransferStateSnapshot>,
}

impl fmt::Debug for MobileFileTransferState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.lock().ok();
        formatter
            .debug_struct("MobileFileTransferState")
            .field("state", &state.as_deref())
            .finish()
    }
}

#[uniffi::export]
impl MobileFileTransferState {
    #[uniffi::constructor]
    pub fn new(encoded: Option<Vec<u8>>) -> Result<Self, MobileFileTransferError> {
        let state = match encoded {
            Some(encoded) => FileTransferStateSnapshot::decode(&encoded)?,
            None => FileTransferStateSnapshot::default(),
        };
        Ok(Self {
            state: Mutex::new(state),
        })
    }

    pub fn match_request(
        &self,
        request: Arc<MobileFileTransferRequest>,
        partial_file_len: Option<u64>,
    ) -> Result<MobileFileTransferRetainedMatch, MobileFileTransferError> {
        let state = self
            .state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        let matched = state.find(request.source_device_id, &request.offer)?;
        Ok(match matched {
            None => MobileFileTransferRetainedMatch {
                kind: MobileFileTransferRetainedKind::New,
                durable_offset: None,
                local_locator: None,
                truncate_to: None,
            },
            Some(FileTransferStateMatch::Partial(partial)) => {
                let truncate_to = match partial_file_len {
                    Some(length) => match partial.recovery_action(length)? {
                        FileTransferRecoveryAction::Keep => None,
                        FileTransferRecoveryAction::TruncateTo(offset) => Some(offset),
                    },
                    None => None,
                };
                MobileFileTransferRetainedMatch {
                    kind: MobileFileTransferRetainedKind::Partial,
                    durable_offset: Some(partial.durable_offset()),
                    local_locator: Some(partial.locator().bytes().to_vec()),
                    truncate_to,
                }
            }
            Some(FileTransferStateMatch::AlreadyComplete(_)) => MobileFileTransferRetainedMatch {
                kind: MobileFileTransferRetainedKind::AlreadyComplete,
                durable_offset: Some(request.offer.file_size()),
                local_locator: None,
                truncate_to: None,
            },
        })
    }

    pub fn upsert_partial(
        &self,
        request: Arc<MobileFileTransferRequest>,
        durable_offset: u64,
        local_locator: Vec<u8>,
        updated_at_unix_secs: u64,
    ) -> Result<(), MobileFileTransferError> {
        let identity =
            FileTransferIdentity::new(request.source_device_id, request.offer.clone());
        let locator = FileTransferLocalLocator::new(local_locator)?;
        let partial =
            FileTransferPartialState::new(identity, durable_offset, locator, updated_at_unix_secs)?;
        self.state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?
            .upsert_partial(partial)
            .map_err(MobileFileTransferError::from)
    }

    pub fn mark_completed(
        &self,
        request: Arc<MobileFileTransferRequest>,
        updated_at_unix_secs: u64,
    ) -> Result<(), MobileFileTransferError> {
        let identity =
            FileTransferIdentity::new(request.source_device_id, request.offer.clone());
        let completed = FileTransferCompletionTombstone::new(identity, updated_at_unix_secs);
        self.state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?
            .mark_completed(completed)
            .map_err(MobileFileTransferError::from)
    }

    pub fn remove(
        &self,
        transfer_id_bytes: Vec<u8>,
    ) -> Result<bool, MobileFileTransferError> {
        let transfer_id = transfer_id(transfer_id_bytes)?;
        Ok(self
            .state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?
            .remove(transfer_id))
    }

    pub fn prune_expired(
        &self,
        now_unix_secs: u64,
        max_age_secs: u64,
    ) -> Result<u64, MobileFileTransferError> {
        let removed = self
            .state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?
            .prune_expired(now_unix_secs, max_age_secs);
        u64::try_from(removed).map_err(|_| MobileFileTransferError::RetainedStateLimit)
    }

    pub fn encode(&self) -> Result<Vec<u8>, MobileFileTransferError> {
        self.state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?
            .encode()
            .map_err(MobileFileTransferError::from)
    }
}

#[derive(uniffi::Object)]
pub struct MobilePreparedFileTransfer {
    offer: FileTransferOffer,
}

impl MobilePreparedFileTransfer {
    pub(crate) fn offer(&self) -> FileTransferOffer {
        self.offer.clone()
    }
}

impl fmt::Debug for MobilePreparedFileTransfer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MobilePreparedFileTransfer")
            .field("offer", &self.offer)
            .finish()
    }
}

#[uniffi::export]
impl MobilePreparedFileTransfer {
    pub fn transfer_id(&self) -> Vec<u8> {
        self.offer.transfer_id().to_bytes().to_vec()
    }

    pub fn display_name(&self) -> String {
        self.offer.display_name().to_owned()
    }

    pub fn file_size(&self) -> u64 {
        self.offer.file_size()
    }

    pub fn verifier(&self) -> Arc<MobileFileTransferVerifier> {
        Arc::new(MobileFileTransferVerifier::new(&self.offer))
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferHasher {
    transfer_id: TransferId,
    hasher: Mutex<Option<FileTransferHasher>>,
}

#[uniffi::export]
impl MobileFileTransferHasher {
    #[uniffi::constructor]
    pub fn new() -> Result<Self, MobileFileTransferError> {
        Ok(Self {
            transfer_id: TransferId::generate().map_err(|_| MobileFileTransferError::Random)?,
            hasher: Mutex::new(Some(FileTransferHasher::new())),
        })
    }

    pub fn update(&self, bytes: Vec<u8>) -> Result<(), MobileFileTransferError> {
        let mut hasher = self
            .hasher
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        hasher
            .as_mut()
            .ok_or(MobileFileTransferError::StateUnavailable)?
            .update(&bytes)
            .map_err(MobileFileTransferError::from)
    }

    pub fn finish(
        &self,
        display_name: String,
    ) -> Result<Arc<MobilePreparedFileTransfer>, MobileFileTransferError> {
        let mut hasher = self
            .hasher
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        let hasher = hasher
            .take()
            .ok_or(MobileFileTransferError::StateUnavailable)?;
        let offer = hasher
            .finish()
            .into_offer(self.transfer_id, display_name)
            .map_err(MobileFileTransferError::from)?;
        Ok(Arc::new(MobilePreparedFileTransfer { offer }))
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferVerifier {
    verifier: Mutex<Option<FileTransferVerifier>>,
}

impl MobileFileTransferVerifier {
    fn new(offer: &FileTransferOffer) -> Self {
        Self {
            verifier: Mutex::new(Some(FileTransferVerifier::new(offer))),
        }
    }
}

#[uniffi::export]
impl MobileFileTransferVerifier {
    pub fn update(&self, bytes: Vec<u8>) -> Result<(), MobileFileTransferError> {
        let mut verifier = self
            .verifier
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        verifier
            .as_mut()
            .ok_or(MobileFileTransferError::StateUnavailable)?
            .update(&bytes)
            .map_err(MobileFileTransferError::from)
    }

    pub fn finish(&self) -> Result<(), MobileFileTransferError> {
        let mut verifier = self
            .verifier
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        let verifier = verifier
            .take()
            .ok_or(MobileFileTransferError::StateUnavailable)?;
        verifier.finish().map_err(MobileFileTransferError::from)
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferSourceStream {
    stream: FileTransferSourceStream,
}

impl MobileFileTransferSourceStream {
    pub(crate) const fn from_agent(stream: FileTransferSourceStream) -> Self {
        Self { stream }
    }

    pub(crate) const fn stream(&self) -> FileTransferSourceStream {
        self.stream
    }
}

#[uniffi::export]
impl MobileFileTransferSourceStream {
    pub fn transfer_id(&self) -> Vec<u8> {
        self.stream.transfer_id().to_bytes().to_vec()
    }

    pub fn resume_offset(&self) -> u64 {
        self.stream.resume_offset()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferChunkOutcome {
    Sent,
    Backpressure,
    TooLarge,
    Closed,
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferChunkResult {
    outcome: MobileFileTransferChunkOutcome,
    chunk: Mutex<Option<Vec<u8>>>,
}

impl MobileFileTransferChunkResult {
    pub(crate) fn sent() -> Self {
        Self {
            outcome: MobileFileTransferChunkOutcome::Sent,
            chunk: Mutex::new(None),
        }
    }

    pub(crate) fn from_error(error: FileTransferChunkError) -> Self {
        let (outcome, chunk) = match error {
            FileTransferChunkError::Backpressure(chunk) => {
                (MobileFileTransferChunkOutcome::Backpressure, Some(chunk))
            }
            FileTransferChunkError::TooLarge(chunk) => {
                (MobileFileTransferChunkOutcome::TooLarge, Some(chunk))
            }
            FileTransferChunkError::Closed(chunk) => {
                (MobileFileTransferChunkOutcome::Closed, chunk)
            }
        };
        Self {
            outcome,
            chunk: Mutex::new(chunk),
        }
    }
}

impl fmt::Debug for MobileFileTransferChunkResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chunk_len = self
            .chunk
            .lock()
            .ok()
            .and_then(|chunk| chunk.as_ref().map(Vec::len));
        formatter
            .debug_struct("MobileFileTransferChunkResult")
            .field("outcome", &self.outcome)
            .field("chunk_len", &chunk_len)
            .finish()
    }
}

#[uniffi::export]
impl MobileFileTransferChunkResult {
    pub fn outcome(&self) -> MobileFileTransferChunkOutcome {
        self.outcome
    }

    pub fn take_chunk(&self) -> Result<Option<Vec<u8>>, MobileFileTransferError> {
        self.chunk
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)
            .map(|mut chunk| chunk.take())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileFileTransferError {
    StateUnavailable,
    InvalidIdentifier,
    InvalidOffer,
    IntegrityFailed,
    RetainedStateInvalid,
    RetainedStateMismatch,
    RetainedStateComplete,
    RetainedStateLimit,
    NotConnected,
    NotNegotiated,
    InvalidResponse,
    InvalidResumeOffset,
    ResourceLimit,
    AlreadyActive,
    InvalidStream,
    Random,
    TimedOut,
    Cancelled,
    Transport,
    Closed,
    RemoteDenied,
    RemoteUnavailable,
    RemoteFailed,
}

impl fmt::Display for MobileFileTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StateUnavailable => "file transfer state is unavailable",
            Self::InvalidIdentifier => "file transfer identifier is invalid",
            Self::InvalidOffer => "file transfer offer is invalid",
            Self::IntegrityFailed => "file transfer integrity verification failed",
            Self::RetainedStateInvalid => "file transfer retained state is invalid",
            Self::RetainedStateMismatch => "file transfer retained identity does not match",
            Self::RetainedStateComplete => "file transfer is already complete",
            Self::RetainedStateLimit => "file transfer retained-state limit is exhausted",
            Self::NotConnected => "file transfer peer is not connected",
            Self::NotNegotiated => "file transfer capability is not negotiated",
            Self::InvalidResponse => "file transfer peer returned an invalid response",
            Self::InvalidResumeOffset => "file transfer resume offset is invalid",
            Self::ResourceLimit => "file transfer capacity is exhausted",
            Self::AlreadyActive => "file transfer is already active",
            Self::InvalidStream => "file transfer stream is invalid",
            Self::Random => "file transfer identifier generation failed",
            Self::TimedOut => "file transfer operation timed out",
            Self::Cancelled => "file transfer operation was cancelled",
            Self::Transport => "file transfer transport failed",
            Self::Closed => "file transfer runtime is closed",
            Self::RemoteDenied => "file transfer was denied by the peer",
            Self::RemoteUnavailable => "file transfer is unavailable on the peer",
            Self::RemoteFailed => "file transfer failed on the peer",
        })
    }
}

impl std::error::Error for MobileFileTransferError {}

impl From<FileTransferOperationError> for MobileFileTransferError {
    fn from(error: FileTransferOperationError) -> Self {
        match error {
            FileTransferOperationError::NotConnected => Self::NotConnected,
            FileTransferOperationError::NotNegotiated => Self::NotNegotiated,
            FileTransferOperationError::InvalidResponse => Self::InvalidResponse,
            FileTransferOperationError::InvalidResumeOffset => Self::InvalidResumeOffset,
            FileTransferOperationError::ResourceLimit => Self::ResourceLimit,
            FileTransferOperationError::AlreadyActive => Self::AlreadyActive,
            FileTransferOperationError::InvalidStream => Self::InvalidStream,
            FileTransferOperationError::Random => Self::Random,
            FileTransferOperationError::TimedOut => Self::TimedOut,
            FileTransferOperationError::Cancelled => Self::Cancelled,
            FileTransferOperationError::Transport => Self::Transport,
            FileTransferOperationError::Closed => Self::Closed,
            FileTransferOperationError::Remote(code) => match code {
                ProtocolErrorCode::AuthorizationDenied
                | ProtocolErrorCode::TrustDenied
                | ProtocolErrorCode::OperationRevoked => Self::RemoteDenied,
                ProtocolErrorCode::CapabilityUnsupported
                | ProtocolErrorCode::CapabilityVersionIncompatible => Self::RemoteUnavailable,
                ProtocolErrorCode::ResourceLimit => Self::ResourceLimit,
                ProtocolErrorCode::Cancelled => Self::Cancelled,
                _ => Self::RemoteFailed,
            },
        }
    }
}

impl From<FileTransferIntegrityError> for MobileFileTransferError {
    fn from(_: FileTransferIntegrityError) -> Self {
        Self::IntegrityFailed
    }
}

impl From<FileTransferProfileError> for MobileFileTransferError {
    fn from(error: FileTransferProfileError) -> Self {
        match error {
            FileTransferProfileError::InvalidResumeOffset => Self::InvalidResumeOffset,
            FileTransferProfileError::InvalidDisplayName
            | FileTransferProfileError::TransferIdMismatch => Self::InvalidOffer,
        }
    }
}

impl From<FileTransferStateError> for MobileFileTransferError {
    fn from(error: FileTransferStateError) -> Self {
        match error {
            FileTransferStateError::IdentityMismatch => Self::RetainedStateMismatch,
            FileTransferStateError::AlreadyComplete => Self::RetainedStateComplete,
            FileTransferStateError::TooManyEntries | FileTransferStateError::SnapshotTooLarge => {
                Self::RetainedStateLimit
            }
            FileTransferStateError::InvalidCheckpoint => Self::InvalidResumeOffset,
            FileTransferStateError::InvalidLocator
            | FileTransferStateError::CheckpointRegression
            | FileTransferStateError::PartialShorterThanCheckpoint
            | FileTransferStateError::UnsupportedSchema
            | FileTransferStateError::MalformedSnapshot
            | FileTransferStateError::SnapshotDigestMismatch
            | FileTransferStateError::DuplicateTransferId
            | FileTransferStateError::InvalidOffer => Self::RetainedStateInvalid,
        }
    }
}

pub(crate) fn request_id(bytes: Vec<u8>) -> Result<RequestId, MobileFileTransferError> {
    let bytes: [u8; 16] = bytes
        .try_into()
        .map_err(|_| MobileFileTransferError::InvalidIdentifier)?;
    Ok(RequestId::from_bytes(bytes))
}

pub(crate) fn stream_id(bytes: Vec<u8>) -> Result<StreamId, MobileFileTransferError> {
    let bytes: [u8; 16] = bytes
        .try_into()
        .map_err(|_| MobileFileTransferError::InvalidIdentifier)?;
    Ok(StreamId::from_bytes(bytes))
}

pub(crate) fn transfer_id(bytes: Vec<u8>) -> Result<TransferId, MobileFileTransferError> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| MobileFileTransferError::InvalidIdentifier)?;
    Ok(TransferId::from_bytes(bytes))
}

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_offer_and_verifier_are_streaming_and_redacted() {
        let hasher = MobileFileTransferHasher::new().unwrap();
        hasher.update(b"hello ".to_vec()).unwrap();
        hasher.update(b"world".to_vec()).unwrap();
        let prepared = hasher.finish("hello.txt".into()).unwrap();

        assert_eq!(prepared.file_size(), 11);
        let verifier = prepared.verifier();
        verifier.update(b"hello ".to_vec()).unwrap();
        verifier.update(b"world".to_vec()).unwrap();
        assert_eq!(verifier.finish(), Ok(()));

        let debug = format!("{prepared:?}");
        assert!(!debug.contains("hello world"));
        assert!(!debug.contains("hello.txt"));
        assert!(debug.contains("display_name_len"));
    }

    #[test]
    fn verifier_rejects_changed_content() {
        let hasher = MobileFileTransferHasher::new().unwrap();
        hasher.update(b"original".to_vec()).unwrap();
        let prepared = hasher.finish("payload.bin".into()).unwrap();

        let verifier = prepared.verifier();
        verifier.update(b"changed!".to_vec()).unwrap();
        assert_eq!(verifier.finish(), Err(MobileFileTransferError::IntegrityFailed));
    }

    #[test]
    fn retained_state_round_trips_partial_identity_and_recovery() {
        let request = Arc::new(MobileFileTransferRequest {
            request_id: RequestId::from_bytes([0x11; 16]),
            source_device_id: DeviceId::from_bytes([0x22; 32]),
            offer: FileTransferOffer::new(
                TransferId::from_bytes([0x33; 32]),
                "payload.bin".into(),
                2_097_152,
                crosslab_protocol::FileTransferDigest::from_bytes([0x44; 32]),
            )
            .unwrap(),
        });
        let state = MobileFileTransferState::new(None).unwrap();
        state
            .upsert_partial(
                Arc::clone(&request),
                1_048_576,
                b"content://private/local".to_vec(),
                10,
            )
            .unwrap();

        let encoded = state.encode().unwrap();
        let restored = MobileFileTransferState::new(Some(encoded)).unwrap();
        let matched = restored
            .match_request(Arc::clone(&request), Some(1_048_600))
            .unwrap();

        assert_eq!(matched.kind, MobileFileTransferRetainedKind::Partial);
        assert_eq!(matched.durable_offset, Some(1_048_576));
        assert_eq!(matched.truncate_to, Some(1_048_576));
        assert_eq!(
            matched.local_locator.as_deref(),
            Some(b"content://private/local".as_slice())
        );
    }

    #[test]
    fn identifiers_require_exact_width() {
        assert_eq!(
            request_id(vec![0; 15]).unwrap_err(),
            MobileFileTransferError::InvalidIdentifier
        );
        assert_eq!(
            stream_id(vec![0; 17]).unwrap_err(),
            MobileFileTransferError::InvalidIdentifier
        );
        assert_eq!(
            transfer_id(vec![0; 31]).unwrap_err(),
            MobileFileTransferError::InvalidIdentifier
        );
    }

    #[test]
    fn chunk_result_debug_redacts_payload() {
        let result = MobileFileTransferChunkResult::from_error(
            FileTransferChunkError::Backpressure(b"private-payload".to_vec()),
        );
        let debug = format!("{result:?}");
        assert!(!debug.contains("private-payload"));
        assert!(debug.contains("chunk_len"));
    }
}
