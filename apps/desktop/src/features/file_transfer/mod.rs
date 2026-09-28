#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{
    FILE_TRANSFER_IO_CHUNK_BYTES, LinuxFileTransferDataAction, LinuxFileTransferError,
    LinuxFileTransferLocator, LinuxFileTransferReceiveFailure, LinuxFileTransferReceiveStatus,
    LinuxFileTransferReceiver, LinuxFileTransferRecovery, LinuxFileTransferRequestAction,
    LinuxFileTransferSendFailure, LinuxFileTransferSendHandle,
    LinuxFileTransferSendStatus, LinuxFileTransferSendToken, LinuxFileTransferService,
    LinuxFileTransferSourceReader,
    LinuxFileTransferStateStore, LinuxIncomingFileTransfer, LinuxPreparedFileSource,
};

#[cfg(target_os = "linux")]
pub(crate) use linux::{
    LinuxFileTransferSendStartError, LinuxFileTransferWorkerError, LinuxFileTransferWorkerHandle,
};
