use core::fmt;
use std::{
    collections::{BTreeSet, VecDeque},
    path::PathBuf,
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use crosslab_agent::{
    FileTransferDataEvent, FileTransferIntegrityError, FileTransferOperationError,
    FileTransferCancellation, TrustedPresenceAgent,
};
use crosslab_protocol::{FileTransferTerminalOutcome, RequestId, StreamId, TransferId};
use tokio::sync::{mpsc, oneshot, watch};

use super::{
    LinuxFileTransferDataAction, LinuxFileTransferError, LinuxFileTransferRequestAction,
    LinuxFileTransferService, LinuxIncomingFileTransfer,
};

const FILE_TRANSFER_SERVICE_CAPACITY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxFileTransferReceiveFailure {
    Connection,
    Integrity,
    Storage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxFileTransferReceiveStatus {
    OfferCancelled {
        request_id: RequestId,
    },
    Ready {
        transfer_id: TransferId,
        resume_offset: u64,
        total_bytes: u64,
    },
    Receiving {
        transfer_id: TransferId,
        received_bytes: u64,
        total_bytes: u64,
    },
    Completed {
        transfer_id: TransferId,
        total_bytes: u64,
    },
    AlreadyComplete {
        transfer_id: TransferId,
        total_bytes: u64,
    },
    Cancelled {
        transfer_id: TransferId,
        received_bytes: u64,
        total_bytes: u64,
    },
    Failed {
        transfer_id: TransferId,
        received_bytes: u64,
        total_bytes: u64,
        failure: LinuxFileTransferReceiveFailure,
    },
}

pub(crate) struct LinuxFileTransferWorkerHandle {
    command_tx: mpsc::Sender<WorkerCommand>,
    stop_tx: watch::Sender<bool>,
}

impl LinuxFileTransferWorkerHandle {
    pub(crate) fn start(
        agent: Arc<TrustedPresenceAgent>,
    ) -> Result<
        (
            Self,
            mpsc::Receiver<LinuxIncomingFileTransfer>,
            mpsc::Receiver<LinuxFileTransferReceiveStatus>,
        ),
        LinuxFileTransferWorkerError,
    > {
        let service = LinuxFileTransferService::from_environment(unix_now_secs())?;
        let requests = agent.take_file_transfer_requests()?;
        let cancellations = agent.take_file_transfer_cancellations()?;
        let data = agent.take_file_transfer_data()?;
        let (command_tx, command_rx) = mpsc::channel(FILE_TRANSFER_SERVICE_CAPACITY);
        let (pending_tx, pending_rx) = mpsc::channel(FILE_TRANSFER_SERVICE_CAPACITY);
        let (status_tx, status_rx) = mpsc::channel(FILE_TRANSFER_SERVICE_CAPACITY);
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
                    agent,
                    service,
                    requests,
                    cancellations,
                    data,
                    command_rx,
                    pending_tx,
                    status_tx,
                    stop_rx,
                ));
            })
            .map_err(|_| LinuxFileTransferWorkerError::Thread)?;

        Ok((
            Self {
                command_tx,
                stop_tx,
            },
            pending_rx,
            status_rx,
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

    pub(crate) async fn decline_offer(
        &self,
        incoming: LinuxIncomingFileTransfer,
    ) -> Result<(), LinuxFileTransferWorkerError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(WorkerCommand::Decline {
                incoming,
                reply: reply_tx,
            })
            .await
            .map_err(|_| LinuxFileTransferWorkerError::Closed)?;
        reply_rx
            .await
            .map_err(|_| LinuxFileTransferWorkerError::Closed)?
    }

    pub(crate) async fn cancel_receive(
        &self,
        transfer_id: TransferId,
    ) -> Result<(), LinuxFileTransferWorkerError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(WorkerCommand::Cancel {
                transfer_id,
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
    Decline {
        incoming: LinuxIncomingFileTransfer,
        reply: oneshot::Sender<Result<(), LinuxFileTransferWorkerError>>,
    },
    Cancel {
        transfer_id: TransferId,
        reply: oneshot::Sender<Result<(), LinuxFileTransferWorkerError>>,
    },
}

#[derive(Default)]
struct PendingOfferTracker {
    visible: BTreeSet<RequestId>,
    cancelled_before_dispatch: VecDeque<RequestId>,
}

impl PendingOfferTracker {
    fn begin_request(&mut self, request_id: RequestId) -> bool {
        let Some(position) = self
            .cancelled_before_dispatch
            .iter()
            .position(|cancelled| *cancelled == request_id)
        else {
            return true;
        };
        self.cancelled_before_dispatch.remove(position);
        false
    }

    fn mark_visible(&mut self, request_id: RequestId) {
        self.visible.insert(request_id);
    }

    fn accept(&mut self, request_id: RequestId) {
        self.visible.remove(&request_id);
    }

    fn cancel(&mut self, request_id: RequestId) -> bool {
        if self.visible.remove(&request_id) {
            return true;
        }
        if self
            .cancelled_before_dispatch
            .iter()
            .any(|cancelled| *cancelled == request_id)
        {
            return false;
        }
        if self.cancelled_before_dispatch.len() >= FILE_TRANSFER_SERVICE_CAPACITY {
            self.cancelled_before_dispatch.pop_front();
        }
        self.cancelled_before_dispatch.push_back(request_id);
        false
    }
}

#[derive(Default)]
struct IgnoredDataStreams {
    stream_ids: VecDeque<StreamId>,
}

impl IgnoredDataStreams {
    fn remember(&mut self, stream_id: StreamId) {
        if self.stream_ids.iter().any(|current| *current == stream_id) {
            return;
        }
        if self.stream_ids.len() >= FILE_TRANSFER_SERVICE_CAPACITY {
            self.stream_ids.pop_front();
        }
        self.stream_ids.push_back(stream_id);
    }

    fn contains(&self, stream_id: StreamId) -> bool {
        self.stream_ids.iter().any(|current| *current == stream_id)
    }
}

async fn run_worker(
    agent: Arc<TrustedPresenceAgent>,
    mut service: LinuxFileTransferService,
    mut requests: mpsc::Receiver<crosslab_agent::FileTransferRequest>,
    mut cancellations: mpsc::Receiver<FileTransferCancellation>,
    mut data: mpsc::Receiver<FileTransferDataEvent>,
    mut commands: mpsc::Receiver<WorkerCommand>,
    pending_tx: mpsc::Sender<LinuxIncomingFileTransfer>,
    status_tx: mpsc::Sender<LinuxFileTransferReceiveStatus>,
    mut stop_rx: watch::Receiver<bool>,
) {
    let mut pending_offers = PendingOfferTracker::default();
    let mut ignored_data_streams = IgnoredDataStreams::default();

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
                if !pending_offers.begin_request(request.request_id()) {
                    continue;
                }
                let transfer_id = request.offer().transfer_id();
                let total_bytes = request.offer().file_size();
                let now = unix_now_secs();
                let action = service.handle_request(
                    request.request_id(),
                    request.source_device_id(),
                    request.into_offer(),
                    now,
                );
                match action {
                    Ok(LinuxFileTransferRequestAction::ChooseDestination(incoming)) => {
                        pending_offers.mark_visible(incoming.request_id());
                        if pending_tx.send(incoming).await.is_err() {
                            return;
                        }
                    }
                    Ok(LinuxFileTransferRequestAction::Ready {
                        request_id,
                        transfer_id,
                        resume_offset,
                    }) => {
                        match agent
                            .complete_file_transfer_ready(request_id, resume_offset)
                            .await
                        {
                            Ok(()) => {
                                let _ = status_tx.send(LinuxFileTransferReceiveStatus::Ready {
                                    transfer_id,
                                    resume_offset,
                                    total_bytes,
                                }).await;
                            }
                            Err(_) => {
                                service.release_transfer(transfer_id);
                                let _ = status_tx.send(LinuxFileTransferReceiveStatus::Failed {
                                    transfer_id,
                                    received_bytes: resume_offset,
                                    total_bytes,
                                    failure: LinuxFileTransferReceiveFailure::Connection,
                                }).await;
                            }
                        }
                    }
                    Ok(LinuxFileTransferRequestAction::AlreadyComplete { request_id, .. }) => {
                        if agent
                            .complete_file_transfer_already_complete(request_id)
                            .await
                            .is_ok()
                        {
                            let _ = status_tx.send(
                                LinuxFileTransferReceiveStatus::AlreadyComplete {
                                    transfer_id,
                                    total_bytes,
                                }
                            ).await;
                        }
                    }
                    Err(error) => {
                        let _ = status_tx.send(LinuxFileTransferReceiveStatus::Failed {
                            transfer_id,
                            received_bytes: 0,
                            total_bytes,
                            failure: map_platform_failure(&error),
                        }).await;
                    }
                }
            }
            cancellation = cancellations.recv() => {
                let Some(cancellation) = cancellation else {
                    return;
                };
                match cancellation {
                    FileTransferCancellation::Request { request_id, .. } => {
                        if pending_offers.cancel(request_id) {
                            let _ = status_tx
                                .send(LinuxFileTransferReceiveStatus::OfferCancelled {
                                    request_id,
                                })
                                .await;
                        }
                    }
                    FileTransferCancellation::Transfer { transfer_id } => {
                        if let Some((received_bytes, total_bytes)) =
                            service.transfer_progress(transfer_id)
                        {
                            service.release_transfer(transfer_id);
                            let _ = status_tx
                                .send(LinuxFileTransferReceiveStatus::Failed {
                                    transfer_id,
                                    received_bytes,
                                    total_bytes,
                                    failure: LinuxFileTransferReceiveFailure::Connection,
                                })
                                .await;
                        }
                    }
                }
            }
            event = data.recv() => {
                let Some(event) = event else {
                    return;
                };
                if ignored_data_streams.contains(event_stream_id(&event)) {
                    continue;
                }
                let transfer_id = event_transfer_id(&event);
                let prior = service.transfer_progress(transfer_id);
                let reports_progress = matches!(
                    &event,
                    FileTransferDataEvent::Opened { .. } | FileTransferDataEvent::Chunk(_)
                );
                match service.handle_data(event, unix_now_secs()) {
                    LinuxFileTransferDataAction::Continue => {
                        if reports_progress
                            && let Some((received_bytes, total_bytes)) =
                                service.transfer_progress(transfer_id)
                        {
                            let _ = status_tx.try_send(
                                LinuxFileTransferReceiveStatus::Receiving {
                                    transfer_id,
                                    received_bytes,
                                    total_bytes,
                                }
                            );
                        }
                    }
                    LinuxFileTransferDataAction::Terminal {
                        transfer_id,
                        outcome,
                    } => {
                        let _ = agent
                            .complete_file_transfer_result(transfer_id, outcome)
                            .await;
                        let (received_bytes, total_bytes) = prior.unwrap_or((0, 0));
                        let status = match outcome {
                            FileTransferTerminalOutcome::Completed => {
                                LinuxFileTransferReceiveStatus::Completed {
                                    transfer_id,
                                    total_bytes,
                                }
                            }
                            FileTransferTerminalOutcome::Cancelled => {
                                LinuxFileTransferReceiveStatus::Cancelled {
                                    transfer_id,
                                    received_bytes,
                                    total_bytes,
                                }
                            }
                            FileTransferTerminalOutcome::IntegrityFailed => {
                                LinuxFileTransferReceiveStatus::Failed {
                                    transfer_id,
                                    received_bytes,
                                    total_bytes,
                                    failure: LinuxFileTransferReceiveFailure::Integrity,
                                }
                            }
                            FileTransferTerminalOutcome::StorageFailed => {
                                LinuxFileTransferReceiveStatus::Failed {
                                    transfer_id,
                                    received_bytes,
                                    total_bytes,
                                    failure: LinuxFileTransferReceiveFailure::Storage,
                                }
                            }
                        };
                        let _ = status_tx.send(status).await;
                    }
                    LinuxFileTransferDataAction::Abort {
                        transfer_id,
                        stream_id,
                        outcome,
                        error,
                    } => {
                        let _ = agent
                            .fail_file_transfer_receive(stream_id, transfer_id, outcome)
                            .await;
                        let (received_bytes, total_bytes) = prior.unwrap_or((0, 0));
                        let _ = status_tx.send(LinuxFileTransferReceiveStatus::Failed {
                            transfer_id,
                            received_bytes,
                            total_bytes,
                            failure: map_platform_failure(&error),
                        }).await;
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
                        pending_offers.accept(incoming.request_id());
                        let result = accept_destination(
                            &agent,
                            &mut service,
                            incoming,
                            final_path,
                        )
                        .await;
                        match result {
                            Ok(status) => {
                                let _ = status_tx.send(status).await;
                                let _ = reply.send(Ok(()));
                            }
                            Err(error) => {
                                let _ = reply.send(Err(error));
                            }
                        }
                    }
                    WorkerCommand::Decline { incoming, reply } => {
                        pending_offers.accept(incoming.request_id());
                        let result = agent
                            .decline_file_transfer_request(incoming.request_id())
                            .await;
                        let result = match result {
                            Ok(()) | Err(FileTransferOperationError::Cancelled) => Ok(()),
                            Err(error) => Err(error.into()),
                        };
                        let _ = reply.send(result);
                    }
                    WorkerCommand::Cancel { transfer_id, reply } => {
                        let result = cancel_receive(
                            &agent,
                            &mut service,
                            &mut ignored_data_streams,
                            transfer_id,
                        )
                        .await;
                        match result {
                            Ok(status) => {
                                let _ = status_tx.send(status).await;
                                let _ = reply.send(Ok(()));
                            }
                            Err(error) => {
                                let _ = reply.send(Err(error));
                            }
                        }
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
) -> Result<LinuxFileTransferReceiveStatus, LinuxFileTransferWorkerError> {
    let total_bytes = incoming.offer().file_size();
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
        Ok(()) => Ok(LinuxFileTransferReceiveStatus::Ready {
            transfer_id,
            resume_offset,
            total_bytes,
        }),
        Err(error) => {
            service.release_transfer(transfer_id);
            Err(error.into())
        }
    }
}

async fn cancel_receive(
    agent: &TrustedPresenceAgent,
    service: &mut LinuxFileTransferService,
    ignored_data_streams: &mut IgnoredDataStreams,
    transfer_id: TransferId,
) -> Result<LinuxFileTransferReceiveStatus, LinuxFileTransferWorkerError> {
    let (local_stream_id, received_bytes, total_bytes) = service.cancel_receive(transfer_id)?;
    if let Some(stream_id) = local_stream_id {
        ignored_data_streams.remember(stream_id);
    }

    match agent.cancel_file_transfer_receive(transfer_id).await {
        Ok(Some(stream_id)) => ignored_data_streams.remember(stream_id),
        Ok(None)
        | Err(
            FileTransferOperationError::NotConnected
            | FileTransferOperationError::InvalidStream
            | FileTransferOperationError::Cancelled
            | FileTransferOperationError::Transport
            | FileTransferOperationError::Closed,
        ) => {}
        Err(error) => return Err(error.into()),
    }
    Ok(LinuxFileTransferReceiveStatus::Cancelled {
        transfer_id,
        received_bytes,
        total_bytes,
    })
}

fn event_stream_id(event: &FileTransferDataEvent) -> StreamId {
    match event {
        FileTransferDataEvent::Opened { stream_id, .. }
        | FileTransferDataEvent::Finished { stream_id, .. }
        | FileTransferDataEvent::Cancelled { stream_id, .. } => *stream_id,
        FileTransferDataEvent::Chunk(chunk) => chunk.stream_id(),
    }
}

fn event_transfer_id(event: &FileTransferDataEvent) -> TransferId {
    match event {
        FileTransferDataEvent::Opened { transfer_id, .. }
        | FileTransferDataEvent::Finished { transfer_id, .. }
        | FileTransferDataEvent::Cancelled { transfer_id, .. } => *transfer_id,
        FileTransferDataEvent::Chunk(chunk) => chunk.transfer_id(),
    }
}

fn map_platform_failure(error: &LinuxFileTransferError) -> LinuxFileTransferReceiveFailure {
    match error {
        LinuxFileTransferError::Integrity(
            FileTransferIntegrityError::SizeMismatch
            | FileTransferIntegrityError::DigestMismatch
            | FileTransferIntegrityError::SizeOverflow,
        ) => LinuxFileTransferReceiveFailure::Integrity,
        _ => LinuxFileTransferReceiveFailure::Storage,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignored_data_streams_are_bounded_and_idempotent() {
        let mut ignored = IgnoredDataStreams::default();
        let repeated = StreamId::from_bytes([0x91; 16]);
        ignored.remember(repeated);
        ignored.remember(repeated);
        assert_eq!(ignored.stream_ids.len(), 1);
        assert!(ignored.contains(repeated));

        for tag in 0..=FILE_TRANSFER_SERVICE_CAPACITY {
            let tag = u8::try_from(tag).expect("file-transfer capacity fits u8");
            ignored.remember(StreamId::from_bytes([tag; 16]));
        }
        assert!(ignored.stream_ids.len() <= FILE_TRANSFER_SERVICE_CAPACITY);
    }

    #[test]
    fn pending_offer_tracker_drops_cancelled_request_before_dispatch() {
        let request_id = RequestId::from_bytes([0x71; 16]);
        let mut tracker = PendingOfferTracker::default();

        assert!(!tracker.cancel(request_id));
        assert!(!tracker.begin_request(request_id));
        assert!(tracker.begin_request(request_id));
    }

    #[test]
    fn pending_offer_tracker_reports_visible_cancellation_once() {
        let request_id = RequestId::from_bytes([0x72; 16]);
        let mut tracker = PendingOfferTracker::default();

        assert!(tracker.begin_request(request_id));
        tracker.mark_visible(request_id);
        assert!(tracker.cancel(request_id));
        assert!(!tracker.cancel(request_id));
    }

    #[test]
    fn accepted_offer_is_not_reported_as_visible_after_cancel_race() {
        let request_id = RequestId::from_bytes([0x73; 16]);
        let mut tracker = PendingOfferTracker::default();

        tracker.mark_visible(request_id);
        tracker.accept(request_id);
        assert!(!tracker.cancel(request_id));
    }
}
