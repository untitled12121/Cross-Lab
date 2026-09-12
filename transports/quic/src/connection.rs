use std::{
    future::Future,
    mem,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
};

use crosslab_core::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, IncomingUniStream,
    StreamAcceptError, StreamOpenError, TransportConnection, TransportSecurityClass,
    TransportSendStream,
};
use quinn::{Connection, RecvStream, SendStream, VarInt};
use tokio::{
    sync::{Semaphore, mpsc, watch},
    task::JoinHandle,
};

use crate::{
    config::QuicTransportConfig,
    record::{read_record, write_record},
    stream::{new_outgoing_uni_stream, run_outgoing_uni_stream, run_uni_acceptor},
};

const CONTROL_FAILURE_CODE: VarInt = VarInt::from_u32(1);
const STREAM_FAILURE_CODE: VarInt = VarInt::from_u32(2);
const LOCAL_CLOSE_CODE: VarInt = VarInt::from_u32(0);

pub(crate) struct SharedState {
    terminal: AtomicBool,
    terminal_tx: watch::Sender<bool>,
}

impl SharedState {
    fn new() -> Self {
        let (terminal_tx, _) = watch::channel(false);
        Self {
            terminal: AtomicBool::new(false),
            terminal_tx,
        }
    }

    pub(crate) fn mark_terminal(&self) -> bool {
        if self.terminal.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.terminal_tx.send_replace(true);
        true
    }

    pub(crate) fn is_terminal(&self) -> bool {
        self.terminal.load(Ordering::Acquire)
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<bool> {
        self.terminal_tx.subscribe()
    }
}

struct TaskRegistryState {
    accepting: bool,
    handles: Vec<JoinHandle<()>>,
}

pub(crate) struct TaskRegistry {
    state: Mutex<TaskRegistryState>,
}

impl TaskRegistry {
    fn new() -> Self {
        Self {
            state: Mutex::new(TaskRegistryState {
                accepting: true,
                handles: Vec::new(),
            }),
        }
    }

    fn spawn<Fut>(&self, future: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut state = lock(&self.state);
        debug_assert!(state.accepting);
        state.handles.push(tokio::spawn(future));
    }

    pub(crate) fn spawn_if_open<Factory, Fut>(&self, factory: Factory) -> bool
    where
        Factory: FnOnce() -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut state = lock(&self.state);
        if !state.accepting {
            return false;
        }
        state.handles.push(tokio::spawn(factory()));
        true
    }

    fn close_and_take(&self) -> Vec<JoinHandle<()>> {
        let mut state = lock(&self.state);
        state.accepting = false;
        mem::take(&mut state.handles)
    }
}

pub struct QuicTransportConnection {
    connection: Connection,
    channel_binding: ChannelBinding,
    metadata: ConnectionMetadata,
    config: QuicTransportConfig,
    shared: Arc<SharedState>,
    outbound_control: mpsc::Sender<Vec<u8>>,
    inbound_control: Mutex<mpsc::Receiver<Vec<u8>>>,
    outgoing_stream_slots: Arc<Semaphore>,
    incoming_streams: Mutex<mpsc::Receiver<IncomingUniStream>>,
    tasks: Arc<TaskRegistry>,
}

impl QuicTransportConnection {
    pub(crate) fn new(
        connection: Connection,
        control_send: SendStream,
        control_recv: RecvStream,
        channel_binding: ChannelBinding,
        metadata: ConnectionMetadata,
        config: QuicTransportConfig,
    ) -> Self {
        let shared = Arc::new(SharedState::new());
        let tasks = Arc::new(TaskRegistry::new());
        let (outbound_control, outbound_rx) = mpsc::channel(config.control_queue_capacity());
        let (inbound_tx, inbound_control) = mpsc::channel(config.control_queue_capacity());
        let (incoming_tx, incoming_streams) =
            mpsc::channel(config.incoming_stream_queue_capacity());
        let outgoing_stream_slots = Arc::new(Semaphore::new(config.outgoing_stream_capacity()));
        let incoming_stream_slots = Arc::new(Semaphore::new(
            config.max_concurrent_remote_uni_streams() as usize,
        ));
        let max_control_frame_bytes = config.max_control_frame_bytes();

        tasks.spawn(run_control_writer(
            connection.clone(),
            Arc::clone(&shared),
            shared.subscribe(),
            outbound_rx,
            control_send,
            max_control_frame_bytes,
        ));
        tasks.spawn(run_control_reader(
            connection.clone(),
            Arc::clone(&shared),
            shared.subscribe(),
            inbound_tx,
            control_recv,
            max_control_frame_bytes,
        ));
        let acceptor_tasks = Arc::clone(&tasks);
        tasks.spawn(run_uni_acceptor(
            connection.clone(),
            Arc::clone(&shared),
            incoming_tx,
            incoming_stream_slots,
            acceptor_tasks,
            config,
        ));
        tasks.spawn(monitor_connection_close(
            connection.clone(),
            Arc::clone(&shared),
        ));

        Self {
            connection,
            channel_binding,
            metadata,
            config,
            shared,
            outbound_control,
            inbound_control: Mutex::new(inbound_control),
            outgoing_stream_slots,
            incoming_streams: Mutex::new(incoming_streams),
            tasks,
        }
    }

    pub async fn shutdown(&self) {
        self.close();
        for task in self.tasks.close_and_take() {
            let _ = task.await;
        }
    }

    fn terminate(&self, code: VarInt, reason: &'static [u8]) {
        if self.shared.mark_terminal() {
            self.connection.close(code, reason);
        }
    }
}

impl TransportConnection for QuicTransportConnection {
    fn security_class(&self) -> TransportSecurityClass {
        TransportSecurityClass::AuthenticatedConfidentialChannel
    }

    fn channel_binding(&self) -> &ChannelBinding {
        &self.channel_binding
    }

    fn connection_metadata(&self) -> &ConnectionMetadata {
        &self.metadata
    }

    fn try_send_control(&self, frame: Vec<u8>) -> Result<(), ControlSendError> {
        if self.shared.is_terminal() {
            return Err(ControlSendError::Closed(frame));
        }
        if frame.len() > self.config.max_control_frame_bytes() {
            return Err(ControlSendError::TooLarge(frame));
        }

        match self.outbound_control.try_send(frame) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(frame)) => Err(ControlSendError::Full(frame)),
            Err(mpsc::error::TrySendError::Closed(frame)) => {
                self.terminate(CONTROL_FAILURE_CODE, b"control bridge closed");
                Err(ControlSendError::Closed(frame))
            }
        }
    }

    fn try_receive_control(&self) -> Result<Vec<u8>, ControlReceiveError> {
        if self.shared.is_terminal() {
            return Err(ControlReceiveError::Closed);
        }

        let result = lock(&self.inbound_control).try_recv();
        match result {
            Ok(frame) => Ok(frame),
            Err(mpsc::error::TryRecvError::Empty) if self.shared.is_terminal() => {
                Err(ControlReceiveError::Closed)
            }
            Err(mpsc::error::TryRecvError::Empty) => Err(ControlReceiveError::Empty),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                self.terminate(CONTROL_FAILURE_CODE, b"control bridge disconnected");
                Err(ControlReceiveError::Closed)
            }
        }
    }

    fn try_open_uni_stream(
        &self,
        opening_frame: Vec<u8>,
    ) -> Result<Box<dyn TransportSendStream>, StreamOpenError> {
        if self.shared.is_terminal() {
            return Err(StreamOpenError::Closed(opening_frame));
        }
        if opening_frame.len() > self.config.max_opening_frame_bytes() {
            return Err(StreamOpenError::TooLarge(opening_frame));
        }

        let permit = match Arc::clone(&self.outgoing_stream_slots).try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => return Err(StreamOpenError::Full(opening_frame)),
        };
        let connection = self.connection.clone();
        let shared = Arc::clone(&self.shared);
        let chunk_capacity = self.config.stream_chunk_queue_capacity();
        let max_chunk_bytes = self.config.max_chunk_bytes();
        let mut opening_frame = Some(opening_frame);
        let mut permit = Some(permit);
        let mut handle = None;

        let spawned = self.tasks.spawn_if_open(|| {
            let (send, driver) = new_outgoing_uni_stream(
                Arc::clone(&shared),
                chunk_capacity,
                max_chunk_bytes,
            );
            handle = Some(Box::new(send) as Box<dyn TransportSendStream>);
            run_outgoing_uni_stream(
                connection.clone(),
                Arc::clone(&shared),
                opening_frame
                    .take()
                    .expect("opening frame is available while spawning"),
                driver,
                permit
                    .take()
                    .expect("outgoing stream permit is available while spawning"),
            )
        });

        if !spawned {
            return Err(StreamOpenError::Closed(
                opening_frame.expect("opening frame remains when spawning is closed"),
            ));
        }
        Ok(handle.expect("send handle is created with its driver"))
    }

    fn try_accept_uni_stream(&self) -> Result<IncomingUniStream, StreamAcceptError> {
        if self.shared.is_terminal() {
            return Err(StreamAcceptError::Closed);
        }

        match lock(&self.incoming_streams).try_recv() {
            Ok(stream) => Ok(stream),
            Err(mpsc::error::TryRecvError::Empty) if self.shared.is_terminal() => {
                Err(StreamAcceptError::Closed)
            }
            Err(mpsc::error::TryRecvError::Empty) => Err(StreamAcceptError::Empty),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                self.terminate(STREAM_FAILURE_CODE, b"stream bridge disconnected");
                Err(StreamAcceptError::Closed)
            }
        }
    }

    fn close(&self) {
        self.terminate(LOCAL_CLOSE_CODE, b"crosslab transport closed");
    }

    fn is_closed(&self) -> bool {
        self.shared.is_terminal()
    }
}

async fn run_control_writer(
    connection: Connection,
    shared: Arc<SharedState>,
    mut terminal: watch::Receiver<bool>,
    mut outbound: mpsc::Receiver<Vec<u8>>,
    mut send: SendStream,
    max_control_frame_bytes: usize,
) {
    loop {
        if *terminal.borrow() {
            return;
        }

        let frame = tokio::select! {
            changed = terminal.changed() => {
                if changed.is_err() || *terminal.borrow() {
                    return;
                }
                continue;
            }
            frame = outbound.recv() => frame,
        };

        let Some(frame) = frame else {
            terminate_shared(
                &connection,
                &shared,
                CONTROL_FAILURE_CODE,
                b"control sender dropped",
            );
            return;
        };
        if write_record(&mut send, &frame, max_control_frame_bytes)
            .await
            .is_err()
        {
            terminate_shared(
                &connection,
                &shared,
                CONTROL_FAILURE_CODE,
                b"control write failed",
            );
            return;
        }
    }
}

async fn run_control_reader(
    connection: Connection,
    shared: Arc<SharedState>,
    mut terminal: watch::Receiver<bool>,
    inbound: mpsc::Sender<Vec<u8>>,
    mut recv: RecvStream,
    max_control_frame_bytes: usize,
) {
    loop {
        if *terminal.borrow() {
            return;
        }

        let frame = tokio::select! {
            changed = terminal.changed() => {
                if changed.is_err() || *terminal.borrow() {
                    return;
                }
                continue;
            }
            frame = read_record(&mut recv, max_control_frame_bytes, false) => {
                match frame {
                    Ok(frame) => frame,
                    Err(_) => {
                        terminate_shared(
                            &connection,
                            &shared,
                            CONTROL_FAILURE_CODE,
                            b"control read failed",
                        );
                        return;
                    }
                }
            }
        };

        tokio::select! {
            changed = terminal.changed() => {
                if changed.is_err() || *terminal.borrow() {
                    return;
                }
            }
            result = inbound.send(frame) => {
                if result.is_err() {
                    terminate_shared(
                        &connection,
                        &shared,
                        CONTROL_FAILURE_CODE,
                        b"control receiver dropped",
                    );
                    return;
                }
            }
        }
    }
}

async fn monitor_connection_close(connection: Connection, shared: Arc<SharedState>) {
    let _ = connection.closed().await;
    shared.mark_terminal();
}

fn terminate_shared(
    connection: &Connection,
    shared: &SharedState,
    code: VarInt,
    reason: &'static [u8],
) {
    if shared.mark_terminal() {
        connection.close(code, reason);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
