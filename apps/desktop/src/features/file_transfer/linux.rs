use core::fmt;

use crosslab_agent::FileTransferStateError;
use crosslab_protocol::FileTransferProfileError;

mod locator;
mod source;
mod state_store;

pub use locator::LinuxFileTransferLocator;
pub use source::{
    FILE_TRANSFER_IO_CHUNK_BYTES, LinuxFileTransferSourceReader, LinuxPreparedFileSource,
};
pub use state_store::LinuxFileTransferStateStore;

#[derive(Debug)]
pub enum LinuxFileTransferError {
    HomeUnavailable,
    InvalidPath,
    InvalidSource,
    SourceChanged,
    InvalidLocator,
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
