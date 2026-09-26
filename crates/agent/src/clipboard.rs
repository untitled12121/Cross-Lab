use core::fmt;

use crosslab_policy::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, LocalCapability, OperationName,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlRequest, ControlResponseResult,
    ProtocolDiagnostic, ProtocolErrorCode, ProtocolFailure, RequestId, RetryClass,
};

pub const CLIPBOARD_TEXT_MAX_BYTES: usize = 65_536;

const CLIPBOARD_READ: &str = "clipboard.read";
const CLIPBOARD_WRITE: &str = "clipboard.write";
const OP_GET: &str = "get";
const OP_SET: &str = "set";
const VERSION: CapabilityVersion = CapabilityVersion::new(1, 0);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClipboardAvailability {
    read: bool,
    write: bool,
}

impl ClipboardAvailability {
    pub const fn new(read: bool, write: bool) -> Self {
        Self { read, write }
    }

    pub const fn read(self) -> bool {
        self.read
    }

    pub const fn write(self) -> bool {
        self.write
    }
}

pub enum ClipboardRequest {
    Read {
        request_id: RequestId,
    },
    Write {
        request_id: RequestId,
        text: String,
    },
}

impl ClipboardRequest {
    pub const fn request_id(&self) -> RequestId {
        match self {
            Self::Read { request_id } | Self::Write { request_id, .. } => *request_id,
        }
    }
}

impl fmt::Debug for ClipboardRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { request_id } => formatter
                .debug_struct("ClipboardRequest::Read")
                .field("request_id", request_id)
                .finish(),
            Self::Write { request_id, text } => formatter
                .debug_struct("ClipboardRequest::Write")
                .field("request_id", request_id)
                .field("text_len", &text.len())
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardPlatformError {
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardOperationError {
    NotConnected,
    NotNegotiated,
    Oversized,
    InvalidResponse,
    ResourceLimit,
    Random,
    TimedOut,
    Cancelled,
    Transport,
    Closed,
    Remote(ProtocolErrorCode),
}

impl fmt::Display for ClipboardOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => formatter.write_str("clipboard peer is not connected"),
            Self::NotNegotiated => formatter.write_str("clipboard capability is not negotiated"),
            Self::Oversized => formatter.write_str("clipboard text exceeds the v1 size limit"),
            Self::InvalidResponse => formatter.write_str("clipboard response violates the v1 profile"),
            Self::ResourceLimit => formatter.write_str("clipboard operation capacity is exhausted"),
            Self::Random => formatter.write_str("clipboard request identifier generation failed"),
            Self::TimedOut => formatter.write_str("clipboard operation timed out"),
            Self::Cancelled => formatter.write_str("clipboard operation was cancelled"),
            Self::Transport => formatter.write_str("clipboard control transport failed"),
            Self::Closed => formatter.write_str("clipboard runtime is closed"),
            Self::Remote(code) => write!(formatter, "clipboard peer returned protocol error {}", code.code()),
        }
    }
}

impl std::error::Error for ClipboardOperationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClipboardKind {
    Read,
    Write,
}

pub(crate) fn local_capabilities(availability: ClipboardAvailability) -> Vec<LocalCapability> {
    let range = CapabilityVersionRange::new(1, 0, 0).expect("clipboard v1 range is valid");
    let mut capabilities = Vec::with_capacity(2);
    if availability.read {
        capabilities.push(LocalCapability::new(
            capability(CLIPBOARD_READ),
            range,
            true,
        ));
    }
    if availability.write {
        capabilities.push(LocalCapability::new(
            capability(CLIPBOARD_WRITE),
            range,
            true,
        ));
    }
    capabilities
}

pub(crate) fn advertisement(availability: ClipboardAvailability) -> CapabilityAdvertisement {
    let mut entries = Vec::with_capacity(2);
    if availability.read {
        entries.push(
            CapabilityAdvertisementEntry::new(
                capability(CLIPBOARD_READ),
                VERSION,
                VERSION,
                true,
            )
            .expect("clipboard read v1 advertisement is valid"),
        );
    }
    if availability.write {
        entries.push(
            CapabilityAdvertisementEntry::new(
                capability(CLIPBOARD_WRITE),
                VERSION,
                VERSION,
                true,
            )
            .expect("clipboard write v1 advertisement is valid"),
        );
    }
    CapabilityAdvertisement::new(entries).expect("clipboard advertisement is bounded")
}

pub(crate) fn read_request(request_id: RequestId) -> ControlRequest {
    ControlRequest::new(
        request_id,
        capability(CLIPBOARD_READ),
        VERSION,
        operation(OP_GET),
        RetryClass::NonRetryable,
        Vec::new(),
    )
}

pub(crate) fn write_request(
    request_id: RequestId,
    text: String,
) -> Result<ControlRequest, ClipboardOperationError> {
    validate_text_len(text.as_bytes())?;
    Ok(ControlRequest::new(
        request_id,
        capability(CLIPBOARD_WRITE),
        VERSION,
        operation(OP_SET),
        RetryClass::NonRetryable,
        text.into_bytes(),
    ))
}

pub(crate) fn decode_inbound(
    request: &ControlRequest,
) -> Result<ClipboardRequest, ControlResponseResult> {
    if request.capability_version() != VERSION {
        return Err(failure(
            ProtocolErrorCode::CapabilityVersionIncompatible,
            "clipboard version is unsupported",
        ));
    }

    match (
        request.capability_id().as_str(),
        request.operation_name().as_str(),
    ) {
        (CLIPBOARD_READ, OP_GET) => {
            if !request.body().is_empty() {
                return Err(failure(
                    ProtocolErrorCode::OperationMismatch,
                    "clipboard read request body must be empty",
                ));
            }
            Ok(ClipboardRequest::Read {
                request_id: request.request_id(),
            })
        }
        (CLIPBOARD_WRITE, OP_SET) => {
            if request.body().len() > CLIPBOARD_TEXT_MAX_BYTES {
                return Err(failure(
                    ProtocolErrorCode::ResourceLimit,
                    "clipboard text exceeds the v1 size limit",
                ));
            }
            let text = core::str::from_utf8(request.body()).map_err(|_| {
                failure(
                    ProtocolErrorCode::UnsupportedMessage,
                    "clipboard text is not valid UTF-8",
                )
            })?;
            Ok(ClipboardRequest::Write {
                request_id: request.request_id(),
                text: text.to_owned(),
            })
        }
        (CLIPBOARD_READ | CLIPBOARD_WRITE, _) => Err(failure(
            ProtocolErrorCode::OperationMismatch,
            "clipboard operation does not match the v1 profile",
        )),
        _ => Err(failure(
            ProtocolErrorCode::CapabilityUnsupported,
            "clipboard capability is unsupported",
        )),
    }
}

pub(crate) fn decode_response(
    kind: ClipboardKind,
    result: &ControlResponseResult,
) -> Result<Option<String>, ClipboardOperationError> {
    match result {
        ControlResponseResult::Error(error) => Err(ClipboardOperationError::Remote(error.code())),
        ControlResponseResult::Success(body) => match kind {
            ClipboardKind::Write if body.is_empty() => Ok(None),
            ClipboardKind::Write => Err(ClipboardOperationError::InvalidResponse),
            ClipboardKind::Read => {
                validate_text_len(body)?;
                let text = core::str::from_utf8(body)
                    .map_err(|_| ClipboardOperationError::InvalidResponse)?;
                Ok(Some(text.to_owned()))
            }
        },
    }
}

pub(crate) fn read_completion(
    result: Result<String, ClipboardPlatformError>,
) -> ControlResponseResult {
    match result {
        Ok(text) if text.len() <= CLIPBOARD_TEXT_MAX_BYTES => {
            ControlResponseResult::Success(text.into_bytes())
        }
        Ok(_) => failure(
            ProtocolErrorCode::ResourceLimit,
            "clipboard text exceeds the v1 size limit",
        ),
        Err(ClipboardPlatformError::Unavailable) => failure(
            ProtocolErrorCode::CapabilityUnsupported,
            "clipboard read is unavailable",
        ),
        Err(ClipboardPlatformError::Failed) => failure(
            ProtocolErrorCode::InternalFailure,
            "clipboard read failed",
        ),
    }
}

pub(crate) fn write_completion(
    result: Result<(), ClipboardPlatformError>,
) -> ControlResponseResult {
    match result {
        Ok(()) => ControlResponseResult::Success(Vec::new()),
        Err(ClipboardPlatformError::Unavailable) => failure(
            ProtocolErrorCode::CapabilityUnsupported,
            "clipboard write is unavailable",
        ),
        Err(ClipboardPlatformError::Failed) => failure(
            ProtocolErrorCode::InternalFailure,
            "clipboard write failed",
        ),
    }
}

pub(crate) fn resource_failure() -> ControlResponseResult {
    failure(
        ProtocolErrorCode::ResourceLimit,
        "clipboard operation capacity is exhausted",
    )
}

pub(crate) fn internal_failure() -> ControlResponseResult {
    failure(
        ProtocolErrorCode::InternalFailure,
        "clipboard platform request is unavailable",
    )
}

pub(crate) fn capability_negotiated(
    negotiated: &[CapabilityId],
    kind: ClipboardKind,
) -> bool {
    let expected = match kind {
        ClipboardKind::Read => CLIPBOARD_READ,
        ClipboardKind::Write => CLIPBOARD_WRITE,
    };
    negotiated.iter().any(|id| id.as_str() == expected)
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("clipboard capability id is canonical")
}

fn operation(value: &str) -> OperationName {
    OperationName::parse(value).expect("clipboard operation name is canonical")
}

fn validate_text_len(bytes: &[u8]) -> Result<(), ClipboardOperationError> {
    if bytes.len() > CLIPBOARD_TEXT_MAX_BYTES {
        return Err(ClipboardOperationError::Oversized);
    }
    Ok(())
}

fn failure(code: ProtocolErrorCode, diagnostic: &'static str) -> ControlResponseResult {
    ControlResponseResult::Error(ProtocolFailure::new(
        code,
        Some(ProtocolDiagnostic::new(diagnostic).expect("static diagnostic is bounded")),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_request_enforces_v1_bound() {
        let request_id = RequestId::from_bytes([0x41; 16]);
        assert!(write_request(request_id, "a".repeat(CLIPBOARD_TEXT_MAX_BYTES)).is_ok());
        assert_eq!(
            write_request(request_id, "a".repeat(CLIPBOARD_TEXT_MAX_BYTES + 1)).unwrap_err(),
            ClipboardOperationError::Oversized
        );
    }

    #[test]
    fn inbound_write_requires_valid_utf8_and_bound() {
        let request = ControlRequest::new(
            RequestId::from_bytes([0x42; 16]),
            capability(CLIPBOARD_WRITE),
            VERSION,
            operation(OP_SET),
            RetryClass::NonRetryable,
            vec![0xff],
        );
        let error = decode_inbound(&request).unwrap_err();
        assert!(matches!(
            error,
            ControlResponseResult::Error(error)
                if error.code() == ProtocolErrorCode::UnsupportedMessage
        ));

        let oversized = ControlRequest::new(
            RequestId::from_bytes([0x43; 16]),
            capability(CLIPBOARD_WRITE),
            VERSION,
            operation(OP_SET),
            RetryClass::NonRetryable,
            vec![b'a'; CLIPBOARD_TEXT_MAX_BYTES + 1],
        );
        let error = decode_inbound(&oversized).unwrap_err();
        assert!(matches!(
            error,
            ControlResponseResult::Error(error)
                if error.code() == ProtocolErrorCode::ResourceLimit
        ));
    }

    #[test]
    fn read_request_requires_empty_body() {
        let request = ControlRequest::new(
            RequestId::from_bytes([0x44; 16]),
            capability(CLIPBOARD_READ),
            VERSION,
            operation(OP_GET),
            RetryClass::NonRetryable,
            b"unexpected".to_vec(),
        );
        let error = decode_inbound(&request).unwrap_err();
        assert!(matches!(
            error,
            ControlResponseResult::Error(error)
                if error.code() == ProtocolErrorCode::OperationMismatch
        ));
    }

    #[test]
    fn clipboard_debug_never_contains_plaintext() {
        let request = ClipboardRequest::Write {
            request_id: RequestId::from_bytes([0x45; 16]),
            text: "super-secret-clipboard".into(),
        };
        let debug = format!("{request:?}");
        assert!(!debug.contains("super-secret-clipboard"));
        assert!(debug.contains("text_len"));
    }

    #[test]
    fn response_decoder_enforces_write_empty_and_read_utf8() {
        assert_eq!(
            decode_response(
                ClipboardKind::Write,
                &ControlResponseResult::Success(b"unexpected".to_vec())
            ),
            Err(ClipboardOperationError::InvalidResponse)
        );
        assert_eq!(
            decode_response(
                ClipboardKind::Read,
                &ControlResponseResult::Success(vec![0xff])
            ),
            Err(ClipboardOperationError::InvalidResponse)
        );
    }
}
