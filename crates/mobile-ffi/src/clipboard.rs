use core::fmt;
use std::sync::Mutex;

use crosslab_agent::{
    ClipboardOperationError, ClipboardPlatformError, ClipboardRequest,
};
use crosslab_protocol::{ProtocolErrorCode, RequestId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileClipboardRequestKind {
    Read,
    Write,
}

#[derive(uniffi::Object)]
pub struct MobileClipboardRequest {
    request_id: RequestId,
    kind: MobileClipboardRequestKind,
    text: Mutex<Option<String>>,
}

impl MobileClipboardRequest {
    pub(crate) fn from_agent(request: ClipboardRequest) -> Self {
        match request {
            ClipboardRequest::Read { request_id } => Self {
                request_id,
                kind: MobileClipboardRequestKind::Read,
                text: Mutex::new(None),
            },
            ClipboardRequest::Write { request_id, text } => Self {
                request_id,
                kind: MobileClipboardRequestKind::Write,
                text: Mutex::new(Some(text)),
            },
        }
    }
}

impl fmt::Debug for MobileClipboardRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text_len = self
            .text
            .lock()
            .ok()
            .and_then(|text| text.as_ref().map(String::len));
        formatter
            .debug_struct("MobileClipboardRequest")
            .field("request_id", &self.request_id)
            .field("kind", &self.kind)
            .field("text_len", &text_len)
            .finish()
    }
}

#[uniffi::export]
impl MobileClipboardRequest {
    pub fn request_id(&self) -> Vec<u8> {
        self.request_id.to_bytes().to_vec()
    }

    pub fn kind(&self) -> MobileClipboardRequestKind {
        self.kind
    }

    pub fn take_text(&self) -> Result<Option<String>, MobileClipboardError> {
        self.text
            .lock()
            .map_err(|_| MobileClipboardError::StateUnavailable)?
            .take()
            .ok_or(MobileClipboardError::PayloadUnavailable)
            .map(Some)
            .or_else(|error| {
                if self.kind == MobileClipboardRequestKind::Read {
                    Ok(None)
                } else {
                    Err(error)
                }
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileClipboardPlatformFailure {
    Unavailable,
    Failed,
}

impl From<MobileClipboardPlatformFailure> for ClipboardPlatformError {
    fn from(failure: MobileClipboardPlatformFailure) -> Self {
        match failure {
            MobileClipboardPlatformFailure::Unavailable => Self::Unavailable,
            MobileClipboardPlatformFailure::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileClipboardError {
    StateUnavailable,
    InvalidRequestId,
    PayloadUnavailable,
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
    RemoteDenied,
    RemoteUnavailable,
    RemoteFailed,
}

impl fmt::Display for MobileClipboardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StateUnavailable => "clipboard state is unavailable",
            Self::InvalidRequestId => "clipboard request identifier is invalid",
            Self::PayloadUnavailable => "clipboard request payload is unavailable",
            Self::NotConnected => "clipboard peer is not connected",
            Self::NotNegotiated => "clipboard capability is not negotiated",
            Self::Oversized => "clipboard text exceeds the v1 size limit",
            Self::InvalidResponse => "clipboard peer returned an invalid response",
            Self::ResourceLimit => "clipboard operation capacity is exhausted",
            Self::Random => "clipboard request identifier generation failed",
            Self::TimedOut => "clipboard operation timed out",
            Self::Cancelled => "clipboard operation was cancelled",
            Self::Transport => "clipboard control transport failed",
            Self::Closed => "clipboard runtime is closed",
            Self::RemoteDenied => "clipboard operation is denied by the peer",
            Self::RemoteUnavailable => "clipboard operation is unavailable on the peer",
            Self::RemoteFailed => "clipboard operation failed on the peer",
        })
    }
}

impl std::error::Error for MobileClipboardError {}

impl From<ClipboardOperationError> for MobileClipboardError {
    fn from(error: ClipboardOperationError) -> Self {
        match error {
            ClipboardOperationError::NotConnected => Self::NotConnected,
            ClipboardOperationError::NotNegotiated => Self::NotNegotiated,
            ClipboardOperationError::Oversized => Self::Oversized,
            ClipboardOperationError::InvalidResponse => Self::InvalidResponse,
            ClipboardOperationError::ResourceLimit => Self::ResourceLimit,
            ClipboardOperationError::Random => Self::Random,
            ClipboardOperationError::TimedOut => Self::TimedOut,
            ClipboardOperationError::Cancelled => Self::Cancelled,
            ClipboardOperationError::Transport => Self::Transport,
            ClipboardOperationError::Closed => Self::Closed,
            ClipboardOperationError::Remote(code) => match code {
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

pub(crate) fn request_id(bytes: Vec<u8>) -> Result<RequestId, MobileClipboardError> {
    let bytes: [u8; 16] = bytes
        .try_into()
        .map_err(|_| MobileClipboardError::InvalidRequestId)?;
    Ok(RequestId::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_debug_redacts_plaintext() {
        let request = MobileClipboardRequest::from_agent(ClipboardRequest::Write {
            request_id: RequestId::from_bytes([0x21; 16]),
            text: "private-mobile-clipboard".into(),
        });

        let debug = format!("{request:?}");
        assert!(!debug.contains("private-mobile-clipboard"));
        assert!(debug.contains("text_len"));
    }

    #[test]
    fn request_id_requires_exact_width() {
        assert_eq!(
            request_id(vec![0u8; 15]).unwrap_err(),
            MobileClipboardError::InvalidRequestId
        );
        assert_eq!(
            request_id(vec![0u8; 17]).unwrap_err(),
            MobileClipboardError::InvalidRequestId
        );
        assert_eq!(
            request_id(vec![0x44; 16]).unwrap().to_bytes(),
            [0x44; 16]
        );
    }

    #[test]
    fn remote_authorization_maps_to_denied_without_diagnostic_payload() {
        assert_eq!(
            MobileClipboardError::from(ClipboardOperationError::Remote(
                ProtocolErrorCode::AuthorizationDenied,
            )),
            MobileClipboardError::RemoteDenied
        );
    }
}
