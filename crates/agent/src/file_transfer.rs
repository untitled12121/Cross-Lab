use core::fmt;

use crosslab_core::EventSubscription;
use crosslab_policy::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, LocalCapability, OperationName,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlRequest, ControlResponseResult,
    Event, EventScope, EventType, FILE_TRANSFER_CAPABILITY_ID, FILE_TRANSFER_RESULT_EVENT_TYPE,
    FileTransferAcceptance, FileTransferOffer, FileTransferResult, FileTransferWireError,
    ProtocolDiagnostic, ProtocolErrorCode, ProtocolFailure, RequestId, RetryClass,
    decode_file_transfer_acceptance, decode_file_transfer_offer, decode_file_transfer_result,
    encode_file_transfer_acceptance, encode_file_transfer_offer,
};

const OP_RECEIVE: &str = "receive";
const VERSION: CapabilityVersion = CapabilityVersion::new(2, 0);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FileTransferAvailability {
    receive: bool,
}

impl FileTransferAvailability {
    pub const fn new(receive: bool) -> Self {
        Self { receive }
    }

    pub const fn receive(self) -> bool {
        self.receive
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
pub enum FileTransferOperationError {
    NotConnected,
    NotNegotiated,
    InvalidResponse,
    ResourceLimit,
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
            Self::NotNegotiated => formatter.write_str("file transfer capability is not negotiated"),
            Self::InvalidResponse => formatter.write_str("file transfer response violates v2"),
            Self::ResourceLimit => formatter.write_str("file transfer operation capacity is exhausted"),
            Self::Random => formatter.write_str("file transfer request identifier generation failed"),
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
    if !availability.receive {
        return Vec::new();
    }
    vec![LocalCapability::new(
        capability(),
        CapabilityVersionRange::new(2, 0, 0).expect("file transfer v2 range is valid"),
        true,
    )]
}

pub(crate) fn advertisement(availability: FileTransferAvailability) -> CapabilityAdvertisement {
    let entries = if availability.receive {
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

pub(crate) fn capability_negotiated(negotiated: &[CapabilityId]) -> bool {
    negotiated
        .iter()
        .any(|id| id.as_str() == FILE_TRANSFER_CAPABILITY_ID)
}

pub(crate) fn offer_request(
    request_id: RequestId,
    offer: &FileTransferOffer,
) -> Result<ControlRequest, FileTransferOperationError> {
    let body = encode_file_transfer_offer(offer).map_err(|_| FileTransferOperationError::InvalidResponse)?;
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
        ControlResponseResult::Error(error) => Err(FileTransferOperationError::Remote(error.code())),
        ControlResponseResult::Success(body) => {
            let acceptance =
                decode_file_transfer_acceptance(body).map_err(|_| FileTransferOperationError::InvalidResponse)?;
            acceptance
                .validate_for_offer(offer)
                .map_err(|_| FileTransferOperationError::InvalidResponse)?;
            Ok(acceptance)
        }
    }
}

pub(crate) fn decode_result_event(event: &Event) -> Option<FileTransferResult> {
    let EventScope::Capability(capability_id) = event.scope() else {
        return None;
    };
    if capability_id.as_str() != FILE_TRANSFER_CAPABILITY_ID
        || event.event_type().as_str() != FILE_TRANSFER_RESULT_EVENT_TYPE
    {
        return None;
    }
    decode_file_transfer_result(event.body()).ok()
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
    CapabilityId::parse(FILE_TRANSFER_CAPABILITY_ID).expect("file transfer capability id is canonical")
}

fn operation() -> OperationName {
    OperationName::parse(OP_RECEIVE).expect("file transfer receive operation is canonical")
}

fn result_event_type() -> EventType {
    EventType::parse(FILE_TRANSFER_RESULT_EVENT_TYPE).expect("file transfer result event is canonical")
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
    use crosslab_protocol::{
        FileTransferDigest, TransferId, encode_file_transfer_offer,
    };

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
    fn receive_capability_is_disabled_by_default() {
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
            RetryClass::Retryable,
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
