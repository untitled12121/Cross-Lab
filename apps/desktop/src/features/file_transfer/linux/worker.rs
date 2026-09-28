use core::fmt;
use std::{
    path::PathBuf,
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use crosslab_agent::{FileTransferOperationError, TrustedPresenceAgent};
use tokio::sync::{mpsc, oneshot, watch};

use super::{
    LinuxFileTransferDataAction, LinuxFileTransferError, LinuxFileTransferRequestAction,
    LinuxFileTransferService, LinuxIncomingFileTransfer,
};

const FILE_TRANSFER_SERVICE_CAPACITY: usize = 8;

pub(crate) struct LinuxFileTransferWorkerHandle {
    command_tx: mpsc::Sender<WorkerCommand>,
    stop_tx: watch::Sender<bool>,
}

impl LinuxFileTransferWorkerHandle {
    pub(crate) fn start(
        agent: Arc<TrustedPresenceAgent>,
    ) -> Result<(Self, mpsc::Receiver<LinuxIncomingFileTransfer>), LinuxFileTransferWorkerError>
    {
        let service = LinuxFileTransferService::from_environment()?;
        let requests = agent.take_file_transfer_requests()?;
        let data = agent.take_file_transfer_data()?;
        let (command_tx, command_rx) = mpsc::channel(FILE_TRANSFER_SERVICE_CAPACITY);
        let (pending_tx, pending_rx) = mpsc::channel(FILE_TRANSFER_SERVICE_CAPACITY);
        let (stop_tx, stop_rx) = watch::channel(false);

        thread::Builder::new()
            .name("crosslab-file-transfer".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                runtime.block_on(run_worker(
                    agent, service, requests, data, command_rx, pending_tx, stop_rx,
                ));
            })
            .map_err(|_| LinuxFileTransferWorkerError::Thread)?;

        Ok((
            Self {
                command_tx,
                stop_tx,
            },
            pending_rx,
        ))
    }

    pub(crate) async fn accept_destination(
        &self,
        incoming: LinuxIncomingFileTransfer,
        final_path: PathBuf,
    ) -> Result<(), LinuxFileTransferWorkerError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(WorkerCommand::Accept {
                incoming,
                final_path,
                reply: reply_tx,
            })
            .await
            .map_err(|_| LinuxFileTransferWorkerError::Closed)?;
        reply_rx
            .await
            .map_err(|_| LinuxFileTransferWorkerError::Closed)?
    }
}

impl Drop for LinuxFileTransferWorkerHandle {
    fn drop(&mut self) {
        self.stop_tx.send_replace(true);
    }
}

enum WorkerCommand {
    Accept {
        incoming: LinuxIncomingFileTransfer,
        final_path: PathBuf,
        reply: oneshot::Sender<Result<(), LinuxFileTransferWorkerError>>,
    },
}

async fn run_worker(
    agent: Arc<TrustedPresenceAgent>,
    mut service: LinuxFileTransferService,
    mut requests: mpsc::Receiver<crosslab_agent::FileTransferRequest>,
    mut data: mpsc::Receiver<crosslab_agent::FileTransferDataEvent>,
    mut commands: mpsc::Receiver<WorkerCommand>,
    pending_tx: mpsc::Sender<LinuxIncomingFileTransfer>,
    mut stop_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_err() || *stop_rx.borrow() {
                    return;
                }
            }
            request = requests.recv() => {
                let Some(request) = request else {
                    return;
                };
                let now = unix_now_secs();
                let action = service.handle_request(
                    request.request_id(),
                    request.source_device_id(),
                    request.into_offer(),
                    now,
                );
                match action {
                    Ok(LinuxFileTransferRequestAction::ChooseDestination(incoming)) => {
                        if pending_tx.send(incoming).await.is_err() {
                            return;
                        }
                    }
                    Ok(LinuxFileTransferRequestAction::Ready {
                        request_id,
                        transfer_id,
                        resume_offset,
                    }) => {
                        if agent
                            .complete_file_transfer_ready(request_id, resume_offset)
                            .await
                            .is_err()
                        {
                            service.release_transfer(transfer_id);
                        }
                    }
                    Ok(LinuxFileTransferRequestAction::AlreadyComplete { request_id, .. }) => {
                        let _ = agent
                            .complete_file_transfer_already_complete(request_id)
                            .await;
                    }
                    Err(_) => {}
                }
            }
            event = data.recv() => {
                let Some(event) = event else {
                    return;
                };
                match service.handle_data(event, unix_now_secs()) {
                    LinuxFileTransferDataAction::Continue => {}
                    LinuxFileTransferDataAction::Terminal {
                        transfer_id,
                        outcome,
                    } => {
                        let _ = agent
                            .complete_file_transfer_result(transfer_id, outcome)
                            .await;
                    }
                    LinuxFileTransferDataAction::Abort {
                        transfer_id,
                        stream_id,
                        outcome,
                        ..
                    } => {
                        let _ = agent.cancel_file_transfer_receive(stream_id).await;
                        let _ = agent
                            .complete_file_transfer_result(transfer_id, outcome)
                            .await;
                    }
                }
            }
            command = commands.recv() => {
                let Some(command) = command else {
                    return;
                };
                match command {
                    WorkerCommand::Accept {
                        incoming,
                        final_path,
                        reply,
                    } => {
                        let result = accept_destination(
                            &agent,
                            &mut service,
                            incoming,
                            final_path,
                        )
                        .await;
                        let _ = reply.send(result);
                    }
                }
            }
        }
    }
}

async fn accept_destination(
    agent: &TrustedPresenceAgent,
    service: &mut LinuxFileTransferService,
    incoming: LinuxIncomingFileTransfer,
    final_path: PathBuf,
) -> Result<(), LinuxFileTransferWorkerError> {
    let action = service.accept_destination(incoming, final_path, unix_now_secs())?;
    let LinuxFileTransferRequestAction::Ready {
        request_id,
        transfer_id,
        resume_offset,
    } = action
    else {
        return Err(LinuxFileTransferWorkerError::InvalidAction);
    };

    match agent
        .complete_file_transfer_ready(request_id, resume_offset)
        .await
    {
        Ok(()) => Ok(()),
        Err(error) => {
            service.release_transfer(transfer_id);
            Err(error.into())
        }
    }
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[derive(Debug)]
pub(crate) enum LinuxFileTransferWorkerError {
    Platform(LinuxFileTransferError),
    Agent(FileTransferOperationError),
    InvalidAction,
    Closed,
    Thread,
}

impl fmt::Display for LinuxFileTransferWorkerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(error) => fmt::Display::fmt(error, formatter),
            Self::Agent(error) => fmt::Display::fmt(error, formatter),
            Self::InvalidAction => {
                formatter.write_str("Linux file-transfer service action is invalid")
            }
            Self::Closed => formatter.write_str("Linux file-transfer service is unavailable"),
            Self::Thread => formatter.write_str("Linux file-transfer service could not start"),
        }
    }
}

impl std::error::Error for LinuxFileTransferWorkerError {}

impl From<LinuxFileTransferError> for LinuxFileTransferWorkerError {
    fn from(error: LinuxFileTransferError) -> Self {
        Self::Platform(error)
    }
}

impl From<FileTransferOperationError> for LinuxFileTransferWorkerError {
    fn from(error: FileTransferOperationError) -> Self {
        Self::Agent(error)
    }
}
