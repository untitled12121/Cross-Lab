use core::fmt;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use crosslab_agent::{
    FileTransferChunkError, FileTransferOperationError, FileTransferSourceStream,
    TrustedPresenceAgent,
};
use crosslab_protocol::{
    FileTransferAcceptance, FileTransferTerminalOutcome, ProtocolErrorCode, TransferId,
};
use tokio::sync::watch;

use super::{LinuxFileTransferError, LinuxPreparedFileSource};

const BACKPRESSURE_RETRY: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxFileTransferSendFailure {
    Source,
    NotConnected,
    NotNegotiated,
    Denied,
    ResourceLimit,
    TimedOut,
    RemoteIntegrity,
    RemoteStorage,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxFileTransferSendStatus {
    Preparing,
    WaitingForPeer {
        total_bytes: u64,
    },
    Transferring {
        transferred_bytes: u64,
        total_bytes: u64,
    },
    Completed {
        total_bytes: u64,
    },
    AlreadyComplete {
        total_bytes: u64,
    },
    Cancelled {
        transferred_bytes: u64,
        total_bytes: u64,
    },
    Failed {
        transferred_bytes: u64,
        total_bytes: u64,
        failure: LinuxFileTransferSendFailure,
    },
}

#[derive(Clone)]
pub struct LinuxFileTransferSendToken {
    transfer_id: TransferId,
    path: PathBuf,
}

impl LinuxFileTransferSendToken {
    fn new(path: PathBuf) -> Result<Self, LinuxFileTransferSendStartError> {
        Ok(Self {
            transfer_id: TransferId::generate()
                .map_err(|_| LinuxFileTransferSendStartError::Random)?,
            path,
        })
    }

    pub const fn transfer_id(&self) -> TransferId {
        self.transfer_id
    }
}

impl fmt::Debug for LinuxFileTransferSendToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxFileTransferSendToken")
            .field("transfer_id", &self.transfer_id)
            .field("path", &"[REDACTED]")
            .finish()
    }
}

pub struct LinuxFileTransferSendHandle {
    cancel_tx: watch::Sender<bool>,
    status: watch::Receiver<LinuxFileTransferSendStatus>,
    token: LinuxFileTransferSendToken,
}

impl LinuxFileTransferSendHandle {
    pub(crate) fn new(
        agent: Arc<TrustedPresenceAgent>,
        path: PathBuf,
        active: Arc<AtomicBool>,
    ) -> Result<Self, LinuxFileTransferSendStartError> {
        Self::start(
            agent,
            LinuxFileTransferSendToken::new(path)?,
            active,
        )
    }

    pub(crate) fn retry(
        agent: Arc<TrustedPresenceAgent>,
        token: LinuxFileTransferSendToken,
        active: Arc<AtomicBool>,
    ) -> Result<Self, LinuxFileTransferSendStartError> {
        Self::start(agent, token, active)
    }

    fn start(
        agent: Arc<TrustedPresenceAgent>,
        token: LinuxFileTransferSendToken,
        active: Arc<AtomicBool>,
    ) -> Result<Self, LinuxFileTransferSendStartError> {
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (status_tx, status) = watch::channel(LinuxFileTransferSendStatus::Preparing);
        let thread_status = status_tx.clone();
        let thread_token = token.clone();

        thread::Builder::new()
            .name("crosslab-file-send".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    let _ = thread_status.send(LinuxFileTransferSendStatus::Failed {
                        transferred_bytes: 0,
                        total_bytes: 0,
                        failure: LinuxFileTransferSendFailure::Failed,
                    });
                    active.store(false, Ordering::Release);
                    return;
                };

                runtime.block_on(run_send(agent, thread_token, cancel_rx, thread_status));
                active.store(false, Ordering::Release);
            })
            .map_err(|_| LinuxFileTransferSendStartError::Thread)?;

        Ok(Self {
            cancel_tx,
            status,
            token,
        })
    }

    pub fn subscribe_status(&self) -> watch::Receiver<LinuxFileTransferSendStatus> {
        self.status.clone()
    }

    pub fn retry_token(&self) -> LinuxFileTransferSendToken {
        self.token.clone()
    }

    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(true);
    }
}

impl Drop for LinuxFileTransferSendHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinuxFileTransferSendStartError {
    Random,
    Thread,
}

impl fmt::Display for LinuxFileTransferSendStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Random => "Linux file-transfer identifier generation failed",
            Self::Thread => "Linux file-transfer sender thread could not start",
        })
    }
}

impl std::error::Error for LinuxFileTransferSendStartError {}

async fn run_send(
    agent: Arc<TrustedPresenceAgent>,
    token: LinuxFileTransferSendToken,
    mut cancel_rx: watch::Receiver<bool>,
    status_tx: watch::Sender<LinuxFileTransferSendStatus>,
) {
    let source = match LinuxPreparedFileSource::prepare_cancellable(
        token.path,
        token.transfer_id,
        || is_cancelled(&cancel_rx),
    ) {
        Ok(Some(source)) => source,
        Ok(None) => {
            cancelled(&status_tx, 0, 0);
            return;
        }
        Err(error) => {
            fail(&status_tx, 0, 0, map_source_error(&error));
            return;
        }
    };
    let total_bytes = source.offer().file_size();

    if is_cancelled(&cancel_rx) {
        cancelled(&status_tx, 0, total_bytes);
        return;
    }
    let _ = status_tx.send(LinuxFileTransferSendStatus::WaitingForPeer { total_bytes });

    let acceptance = tokio::select! {
        _ = wait_cancelled(&mut cancel_rx) => {
            cancelled(&status_tx, 0, total_bytes);
            return;
        }
        result = agent.send_file_offer(source.offer().clone()) => {
            match result {
                Ok(acceptance) => acceptance,
                Err(error) => {
                    fail(&status_tx, 0, total_bytes, map_operation_error(error));
                    return;
                }
            }
        }
    };

    match acceptance {
        FileTransferAcceptance::AlreadyComplete { .. } => {
            let _ = status_tx.send(LinuxFileTransferSendStatus::AlreadyComplete { total_bytes });
        }
        FileTransferAcceptance::Ready { .. } => {
            send_ready(agent, source, total_bytes, cancel_rx, status_tx).await;
        }
    }
}

async fn send_ready(
    agent: Arc<TrustedPresenceAgent>,
    source: LinuxPreparedFileSource,
    total_bytes: u64,
    mut cancel_rx: watch::Receiver<bool>,
    status_tx: watch::Sender<LinuxFileTransferSendStatus>,
) {
    let transfer_id = source.offer().transfer_id();
    let stream = tokio::select! {
        _ = wait_cancelled(&mut cancel_rx) => {
            cancelled(&status_tx, 0, total_bytes);
            return;
        }
        result = agent.open_file_transfer_stream(transfer_id) => {
            match result {
                Ok(stream) => stream,
                Err(error) => {
                    fail(&status_tx, 0, total_bytes, map_operation_error(error));
                    return;
                }
            }
        }
    };

    let mut reader = match source.open_reader(stream.resume_offset()) {
        Ok(reader) => reader,
        Err(error) => {
            let _ = agent.cancel_file_transfer_send(stream).await;
            fail(
                &status_tx,
                stream.resume_offset(),
                total_bytes,
                map_source_error(&error),
            );
            return;
        }
    };
    let mut transferred = stream.resume_offset();
    let _ = status_tx.send(LinuxFileTransferSendStatus::Transferring {
        transferred_bytes: transferred,
        total_bytes,
    });

    loop {
        if is_cancelled(&cancel_rx) {
            cancel_stream(&agent, stream, &status_tx, transferred, total_bytes).await;
            return;
        }

        let chunk = match reader.read_chunk() {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(error) => {
                let _ = agent.cancel_file_transfer_send(stream).await;
                fail(
                    &status_tx,
                    transferred,
                    total_bytes,
                    map_source_error(&error),
                );
                return;
            }
        };
        let chunk_len = u64::try_from(chunk.len()).expect("bounded transfer chunk length fits u64");
        let mut pending = chunk;

        loop {
            if is_cancelled(&cancel_rx) {
                cancel_stream(&agent, stream, &status_tx, transferred, total_bytes).await;
                return;
            }

            match agent.send_file_transfer_chunk(stream, pending).await {
                Ok(()) => break,
                Err(FileTransferChunkError::Backpressure(chunk)) => {
                    pending = chunk;
                    tokio::select! {
                        _ = wait_cancelled(&mut cancel_rx) => {
                            cancel_stream(
                                &agent,
                                stream,
                                &status_tx,
                                transferred,
                                total_bytes,
                            )
                            .await;
                            return;
                        }
                        _ = tokio::time::sleep(BACKPRESSURE_RETRY) => {}
                    }
                }
                Err(FileTransferChunkError::TooLarge(_)
                | FileTransferChunkError::Closed(_)) => {
                    let _ = agent.cancel_file_transfer_send(stream).await;
                    fail(
                        &status_tx,
                        transferred,
                        total_bytes,
                        LinuxFileTransferSendFailure::Failed,
                    );
                    return;
                }
            }
        }

        transferred = transferred.saturating_add(chunk_len).min(total_bytes);
        let _ = status_tx.send(LinuxFileTransferSendStatus::Transferring {
            transferred_bytes: transferred,
            total_bytes,
        });
    }

    match agent.finish_file_transfer_stream(stream).await {
        Ok(result) => match result.outcome() {
            FileTransferTerminalOutcome::Completed => {
                let _ = status_tx.send(LinuxFileTransferSendStatus::Completed { total_bytes });
            }
            FileTransferTerminalOutcome::Cancelled => {
                cancelled(&status_tx, transferred, total_bytes);
            }
            FileTransferTerminalOutcome::IntegrityFailed => {
                fail(
                    &status_tx,
                    transferred,
                    total_bytes,
                    LinuxFileTransferSendFailure::RemoteIntegrity,
                );
            }
            FileTransferTerminalOutcome::StorageFailed => {
                fail(
                    &status_tx,
                    transferred,
                    total_bytes,
                    LinuxFileTransferSendFailure::RemoteStorage,
                );
            }
        },
        Err(error) => fail(
            &status_tx,
            transferred,
            total_bytes,
            map_operation_error(error),
        ),
    }
}

async fn cancel_stream(
    agent: &TrustedPresenceAgent,
    stream: FileTransferSourceStream,
    status_tx: &watch::Sender<LinuxFileTransferSendStatus>,
    transferred_bytes: u64,
    total_bytes: u64,
) {
    let _ = agent.cancel_file_transfer_send(stream).await;
    cancelled(status_tx, transferred_bytes, total_bytes);
}

async fn wait_cancelled(cancel_rx: &mut watch::Receiver<bool>) {
    loop {
        if is_cancelled(cancel_rx) {
            return;
        }
        if cancel_rx.changed().await.is_err() {
            return;
        }
    }
}

fn is_cancelled(cancel_rx: &watch::Receiver<bool>) -> bool {
    *cancel_rx.borrow()
}

fn cancelled(
    status_tx: &watch::Sender<LinuxFileTransferSendStatus>,
    transferred_bytes: u64,
    total_bytes: u64,
) {
    let _ = status_tx.send(LinuxFileTransferSendStatus::Cancelled {
        transferred_bytes,
        total_bytes,
    });
}

fn fail(
    status_tx: &watch::Sender<LinuxFileTransferSendStatus>,
    transferred_bytes: u64,
    total_bytes: u64,
    failure: LinuxFileTransferSendFailure,
) {
    let _ = status_tx.send(LinuxFileTransferSendStatus::Failed {
        transferred_bytes,
        total_bytes,
        failure,
    });
}

fn map_source_error(_: &LinuxFileTransferError) -> LinuxFileTransferSendFailure {
    LinuxFileTransferSendFailure::Source
}

fn map_operation_error(error: FileTransferOperationError) -> LinuxFileTransferSendFailure {
    match error {
        FileTransferOperationError::NotConnected => LinuxFileTransferSendFailure::NotConnected,
        FileTransferOperationError::NotNegotiated => LinuxFileTransferSendFailure::NotNegotiated,
        FileTransferOperationError::ResourceLimit | FileTransferOperationError::AlreadyActive => {
            LinuxFileTransferSendFailure::ResourceLimit
        }
        FileTransferOperationError::TimedOut => LinuxFileTransferSendFailure::TimedOut,
        FileTransferOperationError::Remote(code) => match code {
            ProtocolErrorCode::AuthorizationDenied
            | ProtocolErrorCode::TrustDenied
            | ProtocolErrorCode::OperationRevoked => LinuxFileTransferSendFailure::Denied,
            ProtocolErrorCode::CapabilityUnsupported
            | ProtocolErrorCode::CapabilityVersionIncompatible => {
                LinuxFileTransferSendFailure::NotNegotiated
            }
            ProtocolErrorCode::ResourceLimit => LinuxFileTransferSendFailure::ResourceLimit,
            ProtocolErrorCode::Cancelled => LinuxFileTransferSendFailure::Failed,
            _ => LinuxFileTransferSendFailure::Failed,
        },
        FileTransferOperationError::InvalidResponse
        | FileTransferOperationError::InvalidResumeOffset
        | FileTransferOperationError::InvalidStream
        | FileTransferOperationError::Random
        | FileTransferOperationError::Cancelled
        | FileTransferOperationError::Transport
        | FileTransferOperationError::Closed => LinuxFileTransferSendFailure::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_progress_never_contains_local_path_or_file_name() {
        let status = LinuxFileTransferSendStatus::Failed {
            transferred_bytes: 10,
            total_bytes: 20,
            failure: LinuxFileTransferSendFailure::Source,
        };

        let debug = format!("{status:?}");
        assert!(!debug.contains('/'));
        assert!(!debug.contains("secret.txt"));
    }

    #[test]
    fn authorization_failure_is_presented_as_denied() {
        assert_eq!(
            map_operation_error(FileTransferOperationError::Remote(
                ProtocolErrorCode::AuthorizationDenied,
            )),
            LinuxFileTransferSendFailure::Denied
        );
    }
}
