use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
};

use crosslab_core::{
    IncomingUniStream, StreamReceiveError, StreamSendError, TransportReceiveStream,
    TransportSendStream,
};
use quinn::{Connection, RecvStream, VarInt};
use tokio::sync::{
    OwnedSemaphorePermit, Semaphore,
    mpsc::{self, error::TrySendError},
    watch,
};

use crate::{
    config::QuicTransportConfig,
    connection::{SharedState, TaskRegistry},
    record::{RecordError, read_record, write_record},
};

const STREAM_CANCEL_CODE: VarInt = VarInt::from_u32(0);

const RECEIVE_OPEN: u8 = 0;
const RECEIVE_FINISHED: u8 = 1;
const RECEIVE_CANCELLED: u8 = 2;

struct SendState {
    closed: AtomicBool,
    cancelled: AtomicBool,
}

impl SendState {
    fn new() -> Self {
        Self {
            closed: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.close();
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

struct ReceiveState {
    status: AtomicU8,
}

impl ReceiveState {
    fn new() -> Self {
        Self {
            status: AtomicU8::new(RECEIVE_OPEN),
        }
    }

    fn finish(&self) {
        let _ = self.status.compare_exchange(
            RECEIVE_OPEN,
            RECEIVE_FINISHED,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    fn cancel(&self) {
        self.status.store(RECEIVE_CANCELLED, Ordering::Release);
    }

    fn status(&self) -> u8 {
        self.status.load(Ordering::Acquire)
    }
}

pub(crate) struct QuicSendStream {
    shared: Arc<SharedState>,
    state: Arc<SendState>,
    outbound: Option<mpsc::Sender<Vec<u8>>>,
    cancel_tx: watch::Sender<bool>,
    max_chunk_bytes: usize,
}

pub(crate) struct OutgoingUniDriver {
    state: Arc<SendState>,
    outbound: mpsc::Receiver<Vec<u8>>,
    cancel_rx: watch::Receiver<bool>,
    max_chunk_bytes: usize,
}

pub(crate) fn new_outgoing_uni_stream(
    shared: Arc<SharedState>,
    chunk_capacity: usize,
    max_chunk_bytes: usize,
) -> (QuicSendStream, OutgoingUniDriver) {
    let state = Arc::new(SendState::new());
    let (outbound, outbound_rx) = mpsc::channel(chunk_capacity);
    let (cancel_tx, cancel_rx) = watch::channel(false);

    (
        QuicSendStream {
            shared,
            state: Arc::clone(&state),
            outbound: Some(outbound),
            cancel_tx,
            max_chunk_bytes,
        },
        OutgoingUniDriver {
            state,
            outbound: outbound_rx,
            cancel_rx,
            max_chunk_bytes,
        },
    )
}

impl TransportSendStream for QuicSendStream {
    fn try_send_chunk(&mut self, chunk: Vec<u8>) -> Result<(), StreamSendError> {
        if self.shared.is_terminal() || self.state.is_closed() {
            self.state.close();
            return Err(StreamSendError::Closed(chunk));
        }
        if chunk.len() > self.max_chunk_bytes {
            return Err(StreamSendError::TooLarge(chunk));
        }

        let Some(outbound) = self.outbound.as_ref() else {
            return Err(StreamSendError::Closed(chunk));
        };
        match outbound.try_send(chunk) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(chunk)) => Err(StreamSendError::Full(chunk)),
            Err(TrySendError::Closed(chunk)) => {
                self.state.close();
                Err(StreamSendError::Closed(chunk))
            }
        }
    }

    fn finish(&mut self) {
        self.state.close();
        self.outbound.take();
    }

    fn cancel(&mut self) {
        self.state.cancel();
        self.cancel_tx.send_replace(true);
        self.outbound.take();
    }
}

impl Drop for QuicSendStream {
    fn drop(&mut self) {
        if self.outbound.is_some() && !self.state.is_closed() {
            self.cancel();
        }
    }
}

pub(crate) async fn run_outgoing_uni_stream(
    connection: Connection,
    shared: Arc<SharedState>,
    opening_frame: Vec<u8>,
    mut driver: OutgoingUniDriver,
    _permit: OwnedSemaphorePermit,
) {
    let mut terminal = shared.subscribe();
    if shared.is_terminal() || *driver.cancel_rx.borrow() {
        driver.state.cancel();
        return;
    }

    let open = tokio::select! {
        _ = terminal.changed() => {
            driver.state.close();
            return;
        }
        _ = driver.cancel_rx.changed() => {
            driver.state.cancel();
            return;
        }
        result = connection.open_uni() => result,
    };
    let Ok(mut send) = open else {
        driver.state.close();
        return;
    };

    let stopped = send.stopped();
    tokio::pin!(stopped);
    tokio::select! {
        _ = terminal.changed() => {
            driver.state.close();
            let _ = send.reset(STREAM_CANCEL_CODE);
            return;
        }
        _ = driver.cancel_rx.changed() => {
            driver.state.cancel();
            let _ = send.reset(STREAM_CANCEL_CODE);
            return;
        }
        _ = &mut stopped => {
            driver.state.close();
            return;
        }
        result = write_record(&mut send, &opening_frame, opening_frame.len()) => {
            if result.is_err() {
                driver.state.close();
                return;
            }
        }
    }

    loop {
        if shared.is_terminal() {
            driver.state.close();
            let _ = send.reset(STREAM_CANCEL_CODE);
            return;
        }
        if *driver.cancel_rx.borrow() {
            driver.state.cancel();
            let _ = send.reset(STREAM_CANCEL_CODE);
            return;
        }

        tokio::select! {
            _ = terminal.changed() => {
                driver.state.close();
                let _ = send.reset(STREAM_CANCEL_CODE);
                return;
            }
            _ = driver.cancel_rx.changed() => {
                driver.state.cancel();
                let _ = send.reset(STREAM_CANCEL_CODE);
                return;
            }
            _ = &mut stopped => {
                driver.state.close();
                return;
            }
            chunk = driver.outbound.recv() => {
                let Some(chunk) = chunk else {
                    if driver.state.is_cancelled() {
                        let _ = send.reset(STREAM_CANCEL_CODE);
                    } else {
                        let _ = send.finish();
                    }
                    driver.state.close();
                    return;
                };

                tokio::select! {
                    _ = terminal.changed() => {
                        driver.state.close();
                        let _ = send.reset(STREAM_CANCEL_CODE);
                        return;
                    }
                    _ = driver.cancel_rx.changed() => {
                        driver.state.cancel();
                        let _ = send.reset(STREAM_CANCEL_CODE);
                        return;
                    }
                    _ = &mut stopped => {
                        driver.state.close();
                        return;
                    }
                    result = write_record(&mut send, &chunk, driver.max_chunk_bytes) => {
                        if result.is_err() {
                            driver.state.close();
                            return;
                        }
                    }
                }
            }
        }
    }
}

pub(crate) async fn run_uni_acceptor(
    connection: Connection,
    shared: Arc<SharedState>,
    inbound: mpsc::Sender<IncomingUniStream>,
    incoming_slots: Arc<Semaphore>,
    tasks: Arc<TaskRegistry>,
    config: QuicTransportConfig,
) {
    let mut terminal = shared.subscribe();

    loop {
        if shared.is_terminal() {
            return;
        }

        let permit = tokio::select! {
            _ = terminal.changed() => return,
            result = Arc::clone(&incoming_slots).acquire_owned() => {
                match result {
                    Ok(permit) => permit,
                    Err(_) => return,
                }
            }
        };

        let recv = tokio::select! {
            _ = terminal.changed() => return,
            result = connection.accept_uni() => {
                match result {
                    Ok(recv) => recv,
                    Err(_) => {
                        shared.mark_terminal();
                        return;
                    }
                }
            }
        };

        let child_shared = Arc::clone(&shared);
        let child_inbound = inbound.clone();
        let mut recv = Some(recv);
        let mut permit = Some(permit);
        if !tasks.spawn_if_open(|| {
            run_incoming_uni_stream(
                recv.take().expect("accepted stream is available"),
                child_shared,
                child_inbound,
                config,
                permit.take().expect("incoming stream permit is available"),
            )
        }) {
            return;
        }
    }
}

async fn run_incoming_uni_stream(
    mut recv: RecvStream,
    shared: Arc<SharedState>,
    inbound: mpsc::Sender<IncomingUniStream>,
    config: QuicTransportConfig,
    _permit: OwnedSemaphorePermit,
) {
    let mut terminal = shared.subscribe();
    let opening_frame = tokio::select! {
        _ = terminal.changed() => {
            let _ = recv.stop(STREAM_CANCEL_CODE);
            return;
        }
        result = read_record(&mut recv, config.max_opening_frame_bytes(), false) => result,
    };
    let opening_frame = match opening_frame {
        Ok(frame) => frame,
        Err(RecordError::Finished) => return,
        Err(_) => {
            let _ = recv.stop(STREAM_CANCEL_CODE);
            return;
        }
    };

    let state = Arc::new(ReceiveState::new());
    let (chunks_tx, chunks) = mpsc::channel(config.stream_chunk_queue_capacity());
    let (cancel_tx, mut cancel_rx) = watch::channel(false);
    let incoming = IncomingUniStream::new(
        opening_frame,
        Box::new(QuicReceiveStream {
            shared: Arc::clone(&shared),
            state: Arc::clone(&state),
            chunks,
            cancel_tx,
        }),
    );

    tokio::select! {
        _ = terminal.changed() => {
            state.cancel();
            let _ = recv.stop(STREAM_CANCEL_CODE);
            return;
        }
        result = inbound.send(incoming) => {
            if result.is_err() {
                state.cancel();
                let _ = recv.stop(STREAM_CANCEL_CODE);
                return;
            }
        }
    }

    loop {
        if shared.is_terminal() || *cancel_rx.borrow() {
            state.cancel();
            let _ = recv.stop(STREAM_CANCEL_CODE);
            return;
        }

        let record = tokio::select! {
            _ = terminal.changed() => {
                state.cancel();
                let _ = recv.stop(STREAM_CANCEL_CODE);
                return;
            }
            _ = cancel_rx.changed() => {
                state.cancel();
                let _ = recv.stop(STREAM_CANCEL_CODE);
                return;
            }
            result = read_record(&mut recv, config.max_chunk_bytes(), false) => result,
        };

        let chunk = match record {
            Ok(chunk) => chunk,
            Err(RecordError::Finished) => {
                state.finish();
                return;
            }
            Err(_) => {
                state.cancel();
                let _ = recv.stop(STREAM_CANCEL_CODE);
                return;
            }
        };

        tokio::select! {
            _ = terminal.changed() => {
                state.cancel();
                let _ = recv.stop(STREAM_CANCEL_CODE);
                return;
            }
            _ = cancel_rx.changed() => {
                state.cancel();
                let _ = recv.stop(STREAM_CANCEL_CODE);
                return;
            }
            result = chunks_tx.send(chunk) => {
                if result.is_err() {
                    state.cancel();
                    let _ = recv.stop(STREAM_CANCEL_CODE);
                    return;
                }
            }
        }
    }
}

struct QuicReceiveStream {
    shared: Arc<SharedState>,
    state: Arc<ReceiveState>,
    chunks: mpsc::Receiver<Vec<u8>>,
    cancel_tx: watch::Sender<bool>,
}

impl TransportReceiveStream for QuicReceiveStream {
    fn try_receive_chunk(&mut self) -> Result<Vec<u8>, StreamReceiveError> {
        if self.shared.is_terminal() {
            self.state.cancel();
            return Err(StreamReceiveError::Cancelled);
        }
        if self.state.status() == RECEIVE_CANCELLED {
            return Err(StreamReceiveError::Cancelled);
        }

        match self.chunks.try_recv() {
            Ok(chunk) => Ok(chunk),
            Err(mpsc::error::TryRecvError::Empty) => match self.state.status() {
                RECEIVE_FINISHED => Err(StreamReceiveError::Finished),
                RECEIVE_CANCELLED => Err(StreamReceiveError::Cancelled),
                _ => Err(StreamReceiveError::Empty),
            },
            Err(mpsc::error::TryRecvError::Disconnected) => match self.state.status() {
                RECEIVE_FINISHED => Err(StreamReceiveError::Finished),
                _ => Err(StreamReceiveError::Cancelled),
            },
        }
    }

    fn cancel(&mut self) {
        if self.state.status() != RECEIVE_FINISHED {
            self.state.cancel();
            self.cancel_tx.send_replace(true);
        }
    }
}

impl Drop for QuicReceiveStream {
    fn drop(&mut self) {
        if self.state.status() == RECEIVE_OPEN {
            self.cancel();
        }
    }
}
