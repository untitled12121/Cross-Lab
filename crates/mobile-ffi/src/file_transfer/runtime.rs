use core::fmt;
use std::sync::{Arc, Mutex};

use crosslab_agent::{FileTransferDataEvent, FileTransferRequest, FileTransferSourceStream};
use crosslab_identity::DeviceId;
use crosslab_protocol::{
    FileTransferAcceptance, FileTransferOffer, FileTransferResult, FileTransferTerminalOutcome,
    RequestId, StreamId, TransferId,
};

use super::integrity::MobileFileTransferOffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferAcceptanceKind {
    Ready,
    AlreadyComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferDataKind {
    Opened,
    Chunk,
    Finished,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferChunkOutcome {
    Sent,
    Backpressure,
    TooLarge,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferTerminalOutcome {
    Completed,
    Cancelled,
    IntegrityFailed,
    StorageFailed,
}

impl From<FileTransferTerminalOutcome> for MobileFileTransferTerminalOutcome {
    fn from(value: FileTransferTerminalOutcome) -> Self {
        match value {
            FileTransferTerminalOutcome::Completed => Self::Completed,
            FileTransferTerminalOutcome::Cancelled => Self::Cancelled,
            FileTransferTerminalOutcome::IntegrityFailed => Self::IntegrityFailed,
            FileTransferTerminalOutcome::StorageFailed => Self::StorageFailed,
        }
    }
}

impl From<MobileFileTransferTerminalOutcome> for FileTransferTerminalOutcome {
    fn from(value: MobileFileTransferTerminalOutcome) -> Self {
        match value {
            MobileFileTransferTerminalOutcome::Completed => Self::Completed,
            MobileFileTransferTerminalOutcome::Cancelled => Self::Cancelled,
            MobileFileTransferTerminalOutcome::IntegrityFailed => Self::IntegrityFailed,
            MobileFileTransferTerminalOutcome::StorageFailed => Self::StorageFailed,
        }
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

    pub fn source_device_id(&self) -> Vec<u8> {
        self.source_device_id.as_bytes().to_vec()
    }

    pub fn offer(&self) -> Arc<MobileFileTransferOffer> {
        Arc::new(MobileFileTransferOffer::from_offer(self.offer.clone()))
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferCancellation {
    request_id: Option<RequestId>,
    transfer_id: TransferId,
}

impl MobileFileTransferCancellation {
    pub(crate) fn from_agent(value: crosslab_agent::FileTransferCancellation) -> Self {
        match value {
            crosslab_agent::FileTransferCancellation::Request {
                request_id,
                transfer_id,
            } => Self {
                request_id: Some(request_id),
                transfer_id,
            },
            crosslab_agent::FileTransferCancellation::Transfer { transfer_id } => Self {
                request_id: None,
                transfer_id,
            },
        }
    }
}

#[uniffi::export]
impl MobileFileTransferCancellation {
    pub fn request_id(&self) -> Option<Vec<u8>> {
        self.request_id
            .map(|request_id| request_id.to_bytes().to_vec())
    }

    pub fn transfer_id(&self) -> Vec<u8> {
        self.transfer_id.to_bytes().to_vec()
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferDataEvent {
    kind: MobileFileTransferDataKind,
    transfer_id: TransferId,
    stream_id: StreamId,
    resume_offset: Option<u64>,
    chunk: Mutex<Option<Vec<u8>>>,
}

impl MobileFileTransferDataEvent {
    pub(crate) fn from_agent(value: FileTransferDataEvent) -> Self {
        match value {
            FileTransferDataEvent::Opened {
                transfer_id,
                stream_id,
                resume_offset,
            } => Self {
                kind: MobileFileTransferDataKind::Opened,
                transfer_id,
                stream_id,
                resume_offset: Some(resume_offset),
                chunk: Mutex::new(None),
            },
            FileTransferDataEvent::Chunk(chunk) => Self {
                kind: MobileFileTransferDataKind::Chunk,
                transfer_id: chunk.transfer_id(),
                stream_id: chunk.stream_id(),
                resume_offset: None,
                chunk: Mutex::new(Some(chunk.into_bytes())),
            },
            FileTransferDataEvent::Finished {
                transfer_id,
                stream_id,
            } => Self {
                kind: MobileFileTransferDataKind::Finished,
                transfer_id,
                stream_id,
                resume_offset: None,
                chunk: Mutex::new(None),
            },
            FileTransferDataEvent::Cancelled {
                transfer_id,
                stream_id,
            } => Self {
                kind: MobileFileTransferDataKind::Cancelled,
                transfer_id,
                stream_id,
                resume_offset: None,
                chunk: Mutex::new(None),
            },
        }
    }
}

impl fmt::Debug for MobileFileTransferDataEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chunk_len = self
            .chunk
            .lock()
            .ok()
            .and_then(|chunk| chunk.as_ref().map(Vec::len));
        formatter
            .debug_struct("MobileFileTransferDataEvent")
            .field("kind", &self.kind)
            .field("transfer_id", &self.transfer_id)
            .field("stream_id", &self.stream_id)
            .field("resume_offset", &self.resume_offset)
            .field("chunk_len", &chunk_len)
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

    pub fn take_chunk(&self) -> Result<Option<Vec<u8>>, super::MobileFileTransferError> {
        self.chunk
            .lock()
            .map_err(|_| super::MobileFileTransferError::StateUnavailable)
            .map(|mut chunk| chunk.take())
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferAcceptance {
    kind: MobileFileTransferAcceptanceKind,
    transfer_id: TransferId,
    resume_offset: Option<u64>,
}

impl MobileFileTransferAcceptance {
    pub(crate) fn from_agent(value: FileTransferAcceptance) -> Self {
        match value {
            FileTransferAcceptance::Ready {
                transfer_id,
                resume_offset,
                ..
            } => Self {
                kind: MobileFileTransferAcceptanceKind::Ready,
                transfer_id,
                resume_offset: Some(resume_offset),
            },
            FileTransferAcceptance::AlreadyComplete { transfer_id } => Self {
                kind: MobileFileTransferAcceptanceKind::AlreadyComplete,
                transfer_id,
                resume_offset: None,
            },
        }
    }
}

#[uniffi::export]
impl MobileFileTransferAcceptance {
    pub fn kind(&self) -> MobileFileTransferAcceptanceKind {
        self.kind
    }

    pub fn transfer_id(&self) -> Vec<u8> {
        self.transfer_id.to_bytes().to_vec()
    }

    pub fn resume_offset(&self) -> Option<u64> {
        self.resume_offset
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferSourceStream {
    stream: FileTransferSourceStream,
}

impl MobileFileTransferSourceStream {
    pub(crate) fn from_agent(stream: FileTransferSourceStream) -> Self {
        Self { stream }
    }

    pub(crate) fn stream(&self) -> FileTransferSourceStream {
        self.stream
    }
}

#[uniffi::export]
impl MobileFileTransferSourceStream {
    pub fn transfer_id(&self) -> Vec<u8> {
        self.stream.transfer_id().to_bytes().to_vec()
    }

    pub fn stream_id(&self) -> Vec<u8> {
        self.stream.stream_id().to_bytes().to_vec()
    }

    pub fn resume_offset(&self) -> u64 {
        self.stream.resume_offset()
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferResult {
    result: FileTransferResult,
}

impl MobileFileTransferResult {
    pub(crate) fn from_agent(result: FileTransferResult) -> Self {
        Self { result }
    }
}

#[uniffi::export]
impl MobileFileTransferResult {
    pub fn transfer_id(&self) -> Vec<u8> {
        self.result.transfer_id().to_bytes().to_vec()
    }

    pub fn outcome(&self) -> MobileFileTransferTerminalOutcome {
        self.result.outcome().into()
    }
}
