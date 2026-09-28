use core::fmt;

use crosslab_agent::{FileTransferIntegrityError, FileTransferStateError};
use crosslab_protocol::FileTransferProfileError;

mod locator;
mod receive;
mod sender;
mod service;
mod source;
mod state_store;
mod worker;

pub use locator::LinuxFileTransferLocator;
pub use receive::{LinuxFileTransferReceiver, LinuxFileTransferRecovery};
pub use sender::{
    LinuxFileTransferSendFailure, LinuxFileTransferSendHandle, LinuxFileTransferSendStatus,
    LinuxFileTransferSendToken,
};
pub use service::{
    LinuxFileTransferDataAction, LinuxFileTransferRequestAction, LinuxFileTransferService,
    LinuxIncomingFileTransfer,
};
pub use source::{
    FILE_TRANSFER_IO_CHUNK_BYTES, LinuxFileTransferSourceReader, LinuxPreparedFileSource,
};
pub use state_store::LinuxFileTransferStateStore;
pub(crate) use sender::LinuxFileTransferSendStartError;
pub(crate) use worker::{LinuxFileTransferWorkerError, LinuxFileTransferWorkerHandle};

#[derive(Debug)]
pub enum LinuxFileTransferError {
    HomeUnavailable,
    InvalidPath,
    InvalidSource,
    SourceChanged,
    InvalidLocator,
    InvalidPartial,
    DestinationExists,
    PartialExists,
    RetainedStateMissing,
    RetainedStateExists,
    AlreadyComplete,
    AlreadyActive,
    InvalidStream,
    ChunkTooLarge,
    Integrity(FileTransferIntegrityError),
    State(FileTransferStateError),
    Profile(FileTransferProfileError),
    Io(std::io::Error),
}

impl fmt::Display for LinuxFileTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HomeUnavailable => {
                formatter.write_str("Linux file-transfer state home is unavailable")
            }
            Self::InvalidPath => formatter.write_str("Linux file-transfer path is invalid"),
            Self::InvalidSource => formatter.write_str("selected source is not a regular file"),
            Self::SourceChanged => {
                formatter.write_str("selected source changed during file transfer preparation")
            }
            Self::InvalidLocator => formatter.write_str("Linux file-transfer locator is invalid"),
            Self::InvalidPartial => {
                formatter.write_str("Linux file-transfer partial state is invalid")
            }
            Self::DestinationExists => formatter.write_str("selected destination already exists"),
            Self::PartialExists => {
                formatter.write_str("file-transfer partial target already exists")
            }
            Self::RetainedStateMissing => {
                formatter.write_str("retained file-transfer state is missing")
            }
            Self::RetainedStateExists => {
                formatter.write_str("retained file-transfer state already exists")
            }
            Self::AlreadyComplete => formatter.write_str("file transfer is already complete"),
            Self::AlreadyActive => formatter.write_str("file transfer is already active"),
            Self::InvalidStream => formatter.write_str("file transfer stream state is invalid"),
            Self::ChunkTooLarge => {
                formatter.write_str("file-transfer chunk exceeds the platform I/O bound")
            }
            Self::Integrity(error) => fmt::Display::fmt(error, formatter),
            Self::State(error) => fmt::Display::fmt(error, formatter),
            Self::Profile(error) => fmt::Display::fmt(error, formatter),
            Self::Io(_) => formatter.write_str("Linux file-transfer storage I/O failed"),
        }
    }
}

impl std::error::Error for LinuxFileTransferError {}

impl From<std::io::Error> for LinuxFileTransferError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<FileTransferStateError> for LinuxFileTransferError {
    fn from(error: FileTransferStateError) -> Self {
        Self::State(error)
    }
}

impl From<FileTransferProfileError> for LinuxFileTransferError {
    fn from(error: FileTransferProfileError) -> Self {
        Self::Profile(error)
    }
}

impl From<FileTransferIntegrityError> for LinuxFileTransferError {
    fn from(error: FileTransferIntegrityError) -> Self {
        Self::Integrity(error)
    }
}
