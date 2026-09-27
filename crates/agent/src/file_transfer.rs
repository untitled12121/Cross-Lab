use core::fmt;

use crosslab_core::{EventSubscription, StreamSendError};
use crosslab_policy::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, LocalCapability, OperationId,
    OperationName, SessionId,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlRequest, ControlResponseResult,
    DataStreamOpen, Event, EventId, EventScope, EventType, FILE_TRANSFER_CAPABILITY_ID,
    FILE_TRANSFER_RESULT_EVENT_TYPE, FileTransferAcceptance, FileTransferOffer, FileTransferResult,
    FileTransferTerminalOutcome, FileTransferWireError, ProtocolDiagnostic, ProtocolErrorCode,
    ProtocolFailure, RequestId, RetryClass, StreamDirection, StreamId,
    decode_file_transfer_acceptance, decode_file_transfer_offer, decode_file_transfer_result,
    encode_file_transfer_acceptance, encode_file_transfer_offer, encode_file_transfer_result,
};
use crosslab_runtime::RuntimeActorStreamSendError;

const OP_RECEIVE: &str = "receive";
const VERSION: CapabilityVersion = CapabilityVersion::new(2, 0);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FileTransferAvailability {
    enabled: bool,
}

impl FileTransferAvailability {
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub const fn enabled(self) -> bool {
        self.enabled
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FileTransferRequest {
    request_id: RequestId,
    offer: FileTransferOffer,
}

impl FileTransferRequest {
    pub const fn request_id(&self) -> RequestId {
        self.request_id
    }

    pub const fn offer(&self) -> &FileTransferOffer {
        &self.offer
    }

    pub fn into_offer(self) -> FileTransferOffer {
        self.offer
    }
}

impl fmt::Debug for FileTransferRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferRequest")
            .field("request_id", &self.request_id)
            .field("offer", &self.offer)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTransferSourceStream {
    transfer_id: crosslab_protocol::TransferId,
    stream_id: StreamId,
    resume_offset: u64,
}

impl FileTransferSourceStream {
    pub(crate) const fn new(
        transfer_id: crosslab_protocol::TransferId,
        stream_id: StreamId,
        resume_offset: u64,
    ) -> Self {
        Self {
            transfer_id,
            stream_id,
            resume_offset,
        }
    }

    pub const fn transfer_id(self) -> crosslab_protocol::TransferId {
        self.transfer_id
    }

    pub const fn stream_id(self) -> StreamId {
        self.stream_id
    }

    pub const fn resume_offset(self) -> u64 {
        self.resume_offset
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FileTransferDataChunk {
    transfer_id: crosslab_protocol::TransferId,
    stream_id: StreamId,
    bytes: Vec<u8>,
}

impl FileTransferDataChunk {
    pub(crate) fn new(
        transfer_id: crosslab_protocol::TransferId,
        stream_id: StreamId,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            transfer_id,
            stream_id,
            bytes,
        }
    }

    pub const fn transfer_id(&self) -> crosslab_protocol::TransferId {
        self.transfer_id
    }

    pub const fn stream_id(&self) -> StreamId {
        self.stream_id
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl fmt::Debug for FileTransferDataChunk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferDataChunk")
            .field("transfer_id", &self.transfer_id)
            .field("stream_id", &self.stream_id)
            .field(
                "bytes",
                &format_args!("[REDACTED; {} bytes]", self.bytes.len()),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileTransferDataEvent {
    Opened {
        transfer_id: crosslab_protocol::TransferId,
        stream_id: StreamId,
        resume_offset: u64,
    },
    Chunk(FileTransferDataChunk),
    Finished {
        transfer_id: crosslab_protocol::TransferId,
        stream_id: StreamId,
    },
    Cancelled {
        transfer_id: crosslab_protocol::TransferId,
        stream_id: StreamId,
    },
}

#[derive(PartialEq, Eq)]
pub enum FileTransferChunkError {
    Backpressure(Vec<u8>),
    TooLarge(Vec<u8>),
    Closed(Option<Vec<u8>>),
}

impl FileTransferChunkError {
    pub fn into_chunk(self) -> Option<Vec<u8>> {
        match self {
            Self::Backpressure(chunk) | Self::TooLarge(chunk) => Some(chunk),
            Self::Closed(chunk) => chunk,
        }
    }
}

impl fmt::Debug for FileTransferChunkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backpressure(chunk) => formatter
                .debug_tuple("Backpressure")
                .field(&format_args!("[REDACTED; {} bytes]", chunk.len()))
                .finish(),
            Self::TooLarge(chunk) => formatter
                .debug_tuple("TooLarge")
                .field(&format_args!("[REDACTED; {} bytes]", chunk.len()))
                .finish(),
            Self::Closed(Some(chunk)) => formatter
                .debug_tuple("Closed")
                .field(&format_args!("[REDACTED; {} bytes]", chunk.len()))
                .finish(),
            Self::Closed(None) => formatter
                .debug_tuple("Closed")
                .field(&"[UNAVAILABLE]")
                .finish(),
        }
    }
}

impl fmt::Display for FileTransferChunkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Backpressure(_) => "file transfer data path is applying backpressure",
            Self::TooLarge(_) => "file transfer chunk exceeds the transport limit",
            Self::Closed(_) => "file transfer data stream is closed",
        })
    }
}

impl std::error::Error for FileTransferChunkError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferOperationError {
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
    Remote(ProtocolErrorCode),
}

impl fmt::Display for FileTransferOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => formatter.write_str("file transfer peer is not connected"),
            Self::NotNegotiated => {
                formatter.write_str("file transfer capability is not negotiated")
            }
            Self::InvalidResponse => formatter.write_str("file transfer response violates v2"),
            Self::InvalidResumeOffset => {
                formatter.write_str("file transfer resume offset is invalid")
            }
            Self::ResourceLimit => {
                formatter.write_str("file transfer operation capacity is exhausted")
            }
            Self::AlreadyActive => formatter.write_str("file transfer is already active"),
            Self::InvalidStream => formatter.write_str("file transfer stream is not active"),
            Self::Random => {
                formatter.write_str("file transfer request identifier generation failed")
            }
            Self::TimedOut => formatter.write_str("file transfer offer timed out"),
            Self::Cancelled => formatter.write_str("file transfer offer was cancelled"),
            Self::Transport => formatter.write_str("file transfer control transport failed"),
            Self::Closed => formatter.write_str("file transfer runtime is closed"),
            Self::Remote(code) => write!(
                formatter,
                "file transfer peer returned protocol error {}",
                code.code()
            ),
        }
    }
}

impl std::error::Error for FileTransferOperationError {}

pub(crate) fn local_capabilities(availability: FileTransferAvailability) -> Vec<LocalCapability> {
    if !availability.enabled {
        return Vec::new();
    }
    vec![LocalCapability::new(
        capability(),
        CapabilityVersionRange::new(2, 0, 0).expect("file transfer v2 range is valid"),
        true,
    )]
}

pub(crate) fn advertisement(availability: FileTransferAvailability) -> CapabilityAdvertisement {
    let entries = if availability.enabled {
        vec![
            CapabilityAdvertisementEntry::new(capability(), VERSION, VERSION, true)
                .expect("file transfer v2 advertisement is valid"),
        ]
    } else {
        Vec::new()
    };
    CapabilityAdvertisement::new(entries).expect("file transfer advertisement is bounded")
}

pub(crate) fn result_subscription() -> EventSubscription {
    EventSubscription::new(capability(), result_event_type())
}

pub(crate) fn source_stream_open(
    session_id: SessionId,
    stream_id: StreamId,
    operation_id: OperationId,
) -> DataStreamOpen {
    DataStreamOpen::new(
        session_id,
        stream_id,
        operation_id,
        capability(),
        VERSION,
        operation(),
        StreamDirection::SourceToDestination,
        0,
    )
}

pub(crate) fn terminal_result_event(
    result: FileTransferResult,
) -> Result<Event, FileTransferOperationError> {
    let event_id = EventId::generate().map_err(|_| FileTransferOperationError::Random)?;
    let body = encode_file_transfer_result(result)
        .map_err(|_| FileTransferOperationError::InvalidResponse)?;
    Event::capability(event_id, capability(), result_event_type(), body)
        .map_err(|_| FileTransferOperationError::InvalidResponse)
}

pub(crate) fn decode_terminal_result_event(
    event: &Event,
) -> Result<Option<FileTransferResult>, FileTransferOperationError> {
    let EventScope::Capability(capability_id) = event.scope() else {
        return Ok(None);
    };
    if capability_id.as_str() != FILE_TRANSFER_CAPABILITY_ID
        || event.event_type().as_str() != FILE_TRANSFER_RESULT_EVENT_TYPE
    {
        return Ok(None);
    }
    decode_file_transfer_result(event.body())
        .map(Some)
        .map_err(|_| FileTransferOperationError::InvalidResponse)
}

pub(crate) fn terminal_result(
    transfer_id: crosslab_protocol::TransferId,
    outcome: FileTransferTerminalOutcome,
) -> FileTransferResult {
    FileTransferResult::new(transfer_id, outcome)
}

pub(crate) fn map_chunk_error(error: RuntimeActorStreamSendError) -> FileTransferChunkError {
    match error {
        RuntimeActorStreamSendError::QueueFull(chunk) => {
            FileTransferChunkError::Backpressure(chunk)
        }
        RuntimeActorStreamSendError::Closed(chunk) => FileTransferChunkError::Closed(Some(chunk)),
        RuntimeActorStreamSendError::ActorClosed => FileTransferChunkError::Closed(None),
        RuntimeActorStreamSendError::Stream(StreamSendError::Full(chunk)) => {
            FileTransferChunkError::Backpressure(chunk)
        }
        RuntimeActorStreamSendError::Stream(StreamSendError::TooLarge(chunk)) => {
            FileTransferChunkError::TooLarge(chunk)
        }
        RuntimeActorStreamSendError::Stream(StreamSendError::Closed(chunk)) => {
            FileTransferChunkError::Closed(Some(chunk))
        }
    }
}

pub(crate) fn capability_negotiated(negotiated: &[CapabilityId]) -> bool {
    negotiated
        .iter()
        .any(|id| id.as_str() == FILE_TRANSFER_CAPABILITY_ID)
}

pub(crate) fn offer_request(
    request_id: RequestId,
    offer: &FileTransferOffer,
) -> Result<ControlRequest, FileTransferOperationError> {
    let body = encode_file_transfer_offer(offer)
        .map_err(|_| FileTransferOperationError::InvalidResponse)?;
    Ok(ControlRequest::new(
        request_id,
        capability(),
        VERSION,
        operation(),
        RetryClass::NonRetryable,
        body,
    ))
}

pub(crate) fn decode_inbound(
    request: &ControlRequest,
) -> Result<FileTransferRequest, ControlResponseResult> {
    if request.capability_id().as_str() != FILE_TRANSFER_CAPABILITY_ID {
        return Err(failure(
            ProtocolErrorCode::CapabilityUnsupported,
            "file transfer capability is unsupported",
        ));
    }
    if request.capability_version() != VERSION {
        return Err(failure(
            ProtocolErrorCode::CapabilityVersionIncompatible,
            "file transfer version is unsupported",
        ));
    }
    if request.operation_name().as_str() != OP_RECEIVE
        || request.retry_class() != RetryClass::NonRetryable
    {
        return Err(failure(
            ProtocolErrorCode::OperationMismatch,
            "file transfer operation does not match v2",
        ));
    }

    let offer = decode_file_transfer_offer(request.body()).map_err(wire_failure)?;
    Ok(FileTransferRequest {
        request_id: request.request_id(),
        offer,
    })
}

pub(crate) fn ready_response(
    request: &FileTransferRequest,
    resume_offset: u64,
    operation_id: OperationId,
) -> Result<ControlResponseResult, FileTransferOperationError> {
    request
        .offer
        .validate_resume_offset(resume_offset)
        .map_err(|_| FileTransferOperationError::InvalidResumeOffset)?;
    let acceptance = FileTransferAcceptance::Ready {
        transfer_id: request.offer.transfer_id(),
        resume_offset,
        operation_id,
    };
    let body = encode_file_transfer_acceptance(&acceptance)
        .map_err(|_| FileTransferOperationError::InvalidResponse)?;
    Ok(ControlResponseResult::Success(body))
}

pub(crate) fn already_complete_response(
    request: &FileTransferRequest,
) -> Result<ControlResponseResult, FileTransferOperationError> {
    let acceptance = FileTransferAcceptance::AlreadyComplete {
        transfer_id: request.offer.transfer_id(),
    };
    let body = encode_file_transfer_acceptance(&acceptance)
        .map_err(|_| FileTransferOperationError::InvalidResponse)?;
    Ok(ControlResponseResult::Success(body))
}

pub(crate) fn decode_response(
    offer: &FileTransferOffer,
    result: &ControlResponseResult,
) -> Result<FileTransferAcceptance, FileTransferOperationError> {
    match result {
        ControlResponseResult::Error(error) => {
            Err(FileTransferOperationError::Remote(error.code()))
        }
        ControlResponseResult::Success(body) => {
            let acceptance = decode_file_transfer_acceptance(body)
                .map_err(|_| FileTransferOperationError::InvalidResponse)?;
            acceptance
                .validate_for_offer(offer)
                .map_err(|_| FileTransferOperationError::InvalidResponse)?;
            Ok(acceptance)
        }
    }
}

pub(crate) fn resource_failure() -> ControlResponseResult {
    failure(
        ProtocolErrorCode::ResourceLimit,
        "file transfer operation capacity is exhausted",
    )
}

pub(crate) fn internal_failure() -> ControlResponseResult {
    failure(
        ProtocolErrorCode::InternalFailure,
        "file transfer platform request is unavailable",
    )
}

fn capability() -> CapabilityId {
    CapabilityId::parse(FILE_TRANSFER_CAPABILITY_ID)
        .expect("file transfer capability id is canonical")
}

fn operation() -> OperationName {
    OperationName::parse(OP_RECEIVE).expect("file transfer receive operation is canonical")
}

fn result_event_type() -> EventType {
    EventType::parse(FILE_TRANSFER_RESULT_EVENT_TYPE)
        .expect("file transfer result event is canonical")
}

fn wire_failure(error: FileTransferWireError) -> ControlResponseResult {
    match error {
        FileTransferWireError::PayloadTooLarge { .. } => resource_failure(),
        _ => failure(
            ProtocolErrorCode::UnsupportedMessage,
            "file transfer offer payload is invalid",
        ),
    }
}

fn failure(code: ProtocolErrorCode, diagnostic: &'static str) -> ControlResponseResult {
    ControlResponseResult::Error(ProtocolFailure::new(
        code,
        Some(ProtocolDiagnostic::new(diagnostic).expect("static diagnostic is bounded")),
    ))
}

#[cfg(test)]
mod tests {
    use crosslab_protocol::{FileTransferDigest, TransferId, encode_file_transfer_offer};

    use super::*;

    fn offer() -> FileTransferOffer {
        FileTransferOffer::new(
            TransferId::from_bytes([0x41; 32]),
            "example.txt".into(),
            12,
            FileTransferDigest::from_bytes([0x42; 32]),
        )
        .unwrap()
    }

    #[test]
    fn file_transfer_capability_is_disabled_by_default() {
        assert!(local_capabilities(FileTransferAvailability::default()).is_empty());
        assert!(advertisement(FileTransferAvailability::default()).is_empty());
    }

    #[test]
    fn offer_request_round_trips_without_exposing_paths() {
        let request_id = RequestId::from_bytes([0x43; 16]);
        let request = offer_request(request_id, &offer()).unwrap();
        let decoded = decode_inbound(&request).unwrap();

        assert_eq!(decoded.request_id(), request_id);
        assert_eq!(decoded.offer(), &offer());
        assert!(!format!("{decoded:?}").contains("example.txt"));
    }

    #[test]
    fn inbound_requires_nonretryable_receive_v2() {
        let body = encode_file_transfer_offer(&offer()).unwrap();
        let request = ControlRequest::new(
            RequestId::from_bytes([0x44; 16]),
            capability(),
            VERSION,
            OperationName::parse("send").unwrap(),
            RetryClass::Idempotent,
            body,
        );

        let error = decode_inbound(&request).unwrap_err();
        assert!(matches!(
            error,
            ControlResponseResult::Error(error)
                if error.code() == ProtocolErrorCode::OperationMismatch
        ));
    }

    #[test]
    fn ready_response_validates_resume_checkpoint() {
        let request = FileTransferRequest {
            request_id: RequestId::from_bytes([0x46; 16]),
            offer: FileTransferOffer::new(
                TransferId::from_bytes([0x47; 32]),
                "checkpoint.bin".into(),
                crosslab_protocol::FILE_TRANSFER_CHECKPOINT_BYTES + 9,
                FileTransferDigest::from_bytes([0x48; 32]),
            )
            .unwrap(),
        };
        let operation_id = OperationId::from_bytes([0x49; 32]);

        let response = ready_response(
            &request,
            crosslab_protocol::FILE_TRANSFER_CHECKPOINT_BYTES,
            operation_id,
        )
        .unwrap();
        assert!(matches!(
            decode_response(request.offer(), &response).unwrap(),
            FileTransferAcceptance::Ready {
                resume_offset,
                operation_id: received,
                ..
            } if resume_offset == crosslab_protocol::FILE_TRANSFER_CHECKPOINT_BYTES
                && received == operation_id
        ));
        assert_eq!(
            ready_response(&request, 1, operation_id),
            Err(FileTransferOperationError::InvalidResumeOffset)
        );
    }

    #[test]
    fn source_stream_open_uses_exact_v2_authority_shape() {
        let session_id = SessionId::from_bytes([0x4a; 32]);
        let stream_id = StreamId::from_bytes([0x4b; 16]);
        let operation_id = OperationId::from_bytes([0x4c; 32]);
        let open = source_stream_open(session_id, stream_id, operation_id);

        assert_eq!(open.session_id(), session_id);
        assert_eq!(open.stream_id(), stream_id);
        assert_eq!(open.operation_id(), operation_id);
        assert_eq!(open.capability_id().as_str(), FILE_TRANSFER_CAPABILITY_ID);
        assert_eq!(open.capability_version(), VERSION);
        assert_eq!(open.operation_name().as_str(), OP_RECEIVE);
        assert_eq!(open.direction(), StreamDirection::SourceToDestination);
        assert_eq!(open.stream_index(), 0);
    }

    #[test]
    fn terminal_result_event_round_trips_exact_namespace() {
        let result = FileTransferResult::new(
            offer().transfer_id(),
            FileTransferTerminalOutcome::Completed,
        );
        let event = terminal_result_event(result).unwrap();

        assert_eq!(decode_terminal_result_event(&event).unwrap(), Some(result));
        assert!(!format!("{event:?}").contains("example.txt"));
    }

    #[test]
    fn chunk_errors_preserve_bytes_without_debug_disclosure() {
        let payload = b"private-file-chunk".to_vec();
        let error = map_chunk_error(RuntimeActorStreamSendError::Stream(StreamSendError::Full(
            payload.clone(),
        )));

        assert!(!format!("{error:?}").contains("private-file-chunk"));
        assert_eq!(error.into_chunk(), Some(payload));
    }

    #[test]
    fn already_complete_response_binds_original_transfer() {
        let request = FileTransferRequest {
            request_id: RequestId::from_bytes([0x45; 16]),
            offer: offer(),
        };
        let response = already_complete_response(&request).unwrap();
        let acceptance = decode_response(request.offer(), &response).unwrap();

        assert!(matches!(
            acceptance,
            FileTransferAcceptance::AlreadyComplete { transfer_id }
                if transfer_id == request.offer().transfer_id()
        ));
    }
}
