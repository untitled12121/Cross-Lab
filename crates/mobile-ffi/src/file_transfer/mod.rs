use core::fmt;

use crosslab_agent::{
    FileTransferIntegrityError, FileTransferOperationError, FileTransferStateError,
};
use crosslab_identity::DeviceId;
use crosslab_protocol::{
    FileTransferDigest, FileTransferProfileError, ProtocolErrorCode, RequestId, StreamId,
    TransferId,
};

mod integrity;
mod retained;
mod runtime;

pub use integrity::{
    MobileFileTransferHasher, MobileFileTransferOffer, MobileFileTransferVerifier,
};
pub use retained::{
    MobileExpiredFileTransfer, MobileFileTransferRecovery, MobileFileTransferRecoveryKind,
    MobileFileTransferState,
};
pub use runtime::{
    MobileFileTransferAcceptance, MobileFileTransferAcceptanceKind, MobileFileTransferCancellation,
    MobileFileTransferChunkOutcome, MobileFileTransferDataEvent, MobileFileTransferDataKind,
    MobileFileTransferRequest, MobileFileTransferResult, MobileFileTransferSourceStream,
    MobileFileTransferTerminalOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileFileTransferError {
    StateUnavailable,
    InvalidRequestId,
    InvalidTransferId,
    InvalidStreamId,
    InvalidDeviceId,
    InvalidOffer,
    InvalidResumeOffset,
    NotConnected,
    NotNegotiated,
    InvalidResponse,
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
    Integrity,
    RetainedState,
}

impl fmt::Display for MobileFileTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StateUnavailable => "file transfer state is unavailable",
            Self::InvalidRequestId => "file transfer request identifier is invalid",
            Self::InvalidTransferId => "file transfer identifier is invalid",
            Self::InvalidStreamId => "file transfer stream identifier is invalid",
            Self::InvalidDeviceId => "file transfer device identifier is invalid",
            Self::InvalidOffer => "file transfer offer is invalid",
            Self::InvalidResumeOffset => "file transfer resume offset is invalid",
            Self::NotConnected => "file transfer peer is not connected",
            Self::NotNegotiated => "file transfer capability is not negotiated",
            Self::InvalidResponse => "file transfer peer returned an invalid response",
            Self::ResourceLimit => "file transfer operation capacity is exhausted",
            Self::AlreadyActive => "file transfer is already active",
            Self::InvalidStream => "file transfer stream is not active",
            Self::Random => "file transfer identifier generation failed",
            Self::TimedOut => "file transfer operation timed out",
            Self::Cancelled => "file transfer operation was cancelled",
            Self::Transport => "file transfer transport failed",
            Self::Closed => "file transfer runtime is closed",
            Self::RemoteDenied => "file transfer operation is denied by the peer",
            Self::RemoteUnavailable => "file transfer operation is unavailable on the peer",
            Self::RemoteFailed => "file transfer peer failed the operation",
            Self::Integrity => "file transfer integrity verification failed",
            Self::RetainedState => "file transfer retained state is invalid",
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
        Self::Integrity
    }
}

impl From<FileTransferStateError> for MobileFileTransferError {
    fn from(_: FileTransferStateError) -> Self {
        Self::RetainedState
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

pub(crate) fn request_id(bytes: Vec<u8>) -> Result<RequestId, MobileFileTransferError> {
    let bytes: [u8; 16] = bytes
        .try_into()
        .map_err(|_| MobileFileTransferError::InvalidRequestId)?;
    Ok(RequestId::from_bytes(bytes))
}

pub(crate) fn transfer_id(bytes: Vec<u8>) -> Result<TransferId, MobileFileTransferError> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| MobileFileTransferError::InvalidTransferId)?;
    Ok(TransferId::from_bytes(bytes))
}

pub(crate) fn stream_id(bytes: Vec<u8>) -> Result<StreamId, MobileFileTransferError> {
    let bytes: [u8; 16] = bytes
        .try_into()
        .map_err(|_| MobileFileTransferError::InvalidStreamId)?;
    Ok(StreamId::from_bytes(bytes))
}

pub(super) fn device_id(bytes: Vec<u8>) -> Result<DeviceId, MobileFileTransferError> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| MobileFileTransferError::InvalidDeviceId)?;
    Ok(DeviceId::from_bytes(bytes))
}

pub(super) fn digest(bytes: Vec<u8>) -> Result<FileTransferDigest, MobileFileTransferError> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| MobileFileTransferError::InvalidOffer)?;
    Ok(FileTransferDigest::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_require_exact_widths() {
        assert_eq!(
            request_id(vec![0; 15]).unwrap_err(),
            MobileFileTransferError::InvalidRequestId
        );
        assert_eq!(
            transfer_id(vec![0; 31]).unwrap_err(),
            MobileFileTransferError::InvalidTransferId
        );
        assert_eq!(
            stream_id(vec![0; 17]).unwrap_err(),
            MobileFileTransferError::InvalidStreamId
        );
    }
}
