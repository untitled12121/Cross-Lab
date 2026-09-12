use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, Mutex, MutexGuard},
};

use crosslab_core::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, IncomingUniStream,
    StreamAcceptError, StreamOpenError, StreamReceiveError, StreamSendError, TransportConnection,
    TransportReceiveStream, TransportSecurityClass, TransportSendStream,
};

const CHANNEL_BINDING_PROFILE: &str = "in-process-test";
const DEFAULT_STREAM_CAPACITY: usize = 8;
const DEFAULT_CHUNK_CAPACITY: usize = 8;
const DEFAULT_MAX_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySide {
    A,
    B,
}

impl MemorySide {
    const fn opposite(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryTransportConfig {
    control_capacity: NonZeroUsize,
    stream_capacity: NonZeroUsize,
    chunk_capacity: NonZeroUsize,
    max_chunk_bytes: NonZeroUsize,
}

impl MemoryTransportConfig {
    pub const fn new(
        control_capacity: NonZeroUsize,
        stream_capacity: NonZeroUsize,
        chunk_capacity: NonZeroUsize,
        max_chunk_bytes: NonZeroUsize,
    ) -> Self {
        Self {
            control_capacity,
            stream_capacity,
            chunk_capacity,
            max_chunk_bytes,
        }
    }
}

struct ControlDirectionState {
    queue: VecDeque<Vec<u8>>,
    capacity: usize,
    open: bool,
}

impl ControlDirectionState {
    fn new(capacity: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity),
            capacity,
            open: true,
        }
    }
}

struct MemoryStreamState {
    chunks: VecDeque<Vec<u8>>,
    chunk_capacity: usize,
    max_chunk_bytes: usize,
    send_open: bool,
    cancelled: bool,
    accepted: bool,
}

impl MemoryStreamState {
    fn new(chunk_capacity: usize, max_chunk_bytes: usize) -> Self {
        Self {
            chunks: VecDeque::with_capacity(chunk_capacity),
            chunk_capacity,
            max_chunk_bytes,
            send_open: true,
            cancelled: false,
            accepted: false,
        }
    }

    fn cancel(&mut self) {
        self.cancelled = true;
        self.send_open = false;
        self.chunks.clear();
    }

    fn reclaimable(&self) -> bool {
        self.cancelled || (self.accepted && !self.send_open && self.chunks.is_empty())
    }
}

struct PendingUniStream {
    opening_frame: Vec<u8>,
    state: Arc<Mutex<MemoryStreamState>>,
}

struct DataDirectionState {
    pending: VecDeque<PendingUniStream>,
    live: Vec<Arc<Mutex<MemoryStreamState>>>,
    capacity: usize,
    chunk_capacity: usize,
    max_chunk_bytes: usize,
    open: bool,
}

impl DataDirectionState {
    fn new(capacity: usize, chunk_capacity: usize, max_chunk_bytes: usize) -> Self {
        Self {
            pending: VecDeque::with_capacity(capacity),
            live: Vec::with_capacity(capacity),
            capacity,
            chunk_capacity,
            max_chunk_bytes,
            open: true,
        }
    }

    fn prune_terminal(&mut self) {
        self.pending
            .retain(|pending| !lock_stream(&pending.state).cancelled);
        self.live.retain(|state| !lock_stream(state).reclaimable());
    }

    fn cancel_all(&mut self) {
        for state in &self.live {
            lock_stream(state).cancel();
        }
        self.pending.clear();
        self.live.clear();
        self.open = false;
    }
}

struct MemoryState {
    control_a_to_b: ControlDirectionState,
    control_b_to_a: ControlDirectionState,
    data_a_to_b: DataDirectionState,
    data_b_to_a: DataDirectionState,
    closed: bool,
}

impl MemoryState {
    fn new(config: MemoryTransportConfig) -> Self {
        Self {
            control_a_to_b: ControlDirectionState::new(config.control_capacity.get()),
            control_b_to_a: ControlDirectionState::new(config.control_capacity.get()),
            data_a_to_b: DataDirectionState::new(
                config.stream_capacity.get(),
                config.chunk_capacity.get(),
                config.max_chunk_bytes.get(),
            ),
            data_b_to_a: DataDirectionState::new(
                config.stream_capacity.get(),
                config.chunk_capacity.get(),
                config.max_chunk_bytes.get(),
            ),
            closed: false,
        }
    }

    fn control_outbound_mut(&mut self, side: MemorySide) -> &mut ControlDirectionState {
        match side {
            MemorySide::A => &mut self.control_a_to_b,
            MemorySide::B => &mut self.control_b_to_a,
        }
    }

    fn control_inbound_mut(&mut self, side: MemorySide) -> &mut ControlDirectionState {
        self.control_outbound_mut(side.opposite())
    }

    fn data_outbound_mut(&mut self, side: MemorySide) -> &mut DataDirectionState {
        match side {
            MemorySide::A => &mut self.data_a_to_b,
            MemorySide::B => &mut self.data_b_to_a,
        }
    }

    fn data_inbound_mut(&mut self, side: MemorySide) -> &mut DataDirectionState {
        self.data_outbound_mut(side.opposite())
    }

    fn close_outbound(&mut self, side: MemorySide) {
        self.control_outbound_mut(side).open = false;
        self.data_outbound_mut(side).cancel_all();
    }

    fn close_all(&mut self) {
        self.closed = true;
        self.control_a_to_b.open = false;
        self.control_b_to_a.open = false;
        self.control_a_to_b.queue.clear();
        self.control_b_to_a.queue.clear();
        self.data_a_to_b.cancel_all();
        self.data_b_to_a.cancel_all();
    }
}

pub struct MemoryTransportEndpoint {
    side: MemorySide,
    state: Arc<Mutex<MemoryState>>,
    channel_binding: ChannelBinding,
    metadata: ConnectionMetadata,
}

impl MemoryTransportEndpoint {
    fn new(
        side: MemorySide,
        state: Arc<Mutex<MemoryState>>,
        channel_binding: ChannelBinding,
    ) -> Self {
        let (local_endpoint, remote_endpoint) = match side {
            MemorySide::A => ("memory:a", "memory:b"),
            MemorySide::B => ("memory:b", "memory:a"),
        };

        Self {
            side,
            state,
            channel_binding,
            metadata: ConnectionMetadata::new(
                Some(local_endpoint.to_owned()),
                Some(remote_endpoint.to_owned()),
                Some(false),
            ),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, MemoryState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl TransportConnection for MemoryTransportEndpoint {
    fn security_class(&self) -> TransportSecurityClass {
        TransportSecurityClass::InProcessTest
    }

    fn channel_binding(&self) -> &ChannelBinding {
        &self.channel_binding
    }

    fn connection_metadata(&self) -> &ConnectionMetadata {
        &self.metadata
    }

    fn try_send_control(&self, frame: Vec<u8>) -> Result<(), ControlSendError> {
        let mut state = self.lock_state();
        if state.closed {
            return Err(ControlSendError::Closed(frame));
        }

        let outbound = state.control_outbound_mut(self.side);
        if !outbound.open {
            return Err(ControlSendError::Closed(frame));
        }
        if outbound.queue.len() >= outbound.capacity {
            return Err(ControlSendError::Full(frame));
        }

        outbound.queue.push_back(frame);
        Ok(())
    }

    fn try_receive_control(&self) -> Result<Vec<u8>, ControlReceiveError> {
        let mut state = self.lock_state();
        if state.closed {
            return Err(ControlReceiveError::Closed);
        }

        let inbound = state.control_inbound_mut(self.side);
        if let Some(frame) = inbound.queue.pop_front() {
            return Ok(frame);
        }
        if !inbound.open {
            return Err(ControlReceiveError::Closed);
        }

        Err(ControlReceiveError::Empty)
    }

    fn try_open_uni_stream(
        &self,
        opening_frame: Vec<u8>,
    ) -> Result<Box<dyn TransportSendStream>, StreamOpenError> {
        let mut connection = self.lock_state();
        if connection.closed {
            return Err(StreamOpenError::Closed(opening_frame));
        }

        let outbound = connection.data_outbound_mut(self.side);
        outbound.prune_terminal();
        if !outbound.open {
            return Err(StreamOpenError::Closed(opening_frame));
        }
        if outbound.pending.len() >= outbound.capacity || outbound.live.len() >= outbound.capacity {
            return Err(StreamOpenError::Full(opening_frame));
        }

        let stream = Arc::new(Mutex::new(MemoryStreamState::new(
            outbound.chunk_capacity,
            outbound.max_chunk_bytes,
        )));
        outbound.pending.push_back(PendingUniStream {
            opening_frame,
            state: Arc::clone(&stream),
        });
        outbound.live.push(Arc::clone(&stream));
        Ok(Box::new(MemorySendStream { state: stream }))
    }

    fn try_accept_uni_stream(&self) -> Result<IncomingUniStream, StreamAcceptError> {
        let mut connection = self.lock_state();
        if connection.closed {
            return Err(StreamAcceptError::Closed);
        }

        let inbound = connection.data_inbound_mut(self.side);
        inbound.prune_terminal();
        loop {
            let Some(pending) = inbound.pending.pop_front() else {
                return if inbound.open {
                    Err(StreamAcceptError::Empty)
                } else {
                    Err(StreamAcceptError::Closed)
                };
            };
            let mut stream = lock_stream(&pending.state);
            if stream.cancelled {
                continue;
            }
            stream.accepted = true;
            drop(stream);
            return Ok(IncomingUniStream::new(
                pending.opening_frame,
                Box::new(MemoryReceiveStream {
                    state: pending.state,
                }),
            ));
        }
    }

    fn close(&self) {
        self.lock_state().close_all();
    }

    fn is_closed(&self) -> bool {
        self.lock_state().closed
    }
}

struct MemorySendStream {
    state: Arc<Mutex<MemoryStreamState>>,
}

impl TransportSendStream for MemorySendStream {
    fn try_send_chunk(&mut self, chunk: Vec<u8>) -> Result<(), StreamSendError> {
        let mut state = lock_stream(&self.state);
        if state.cancelled || !state.send_open {
            return Err(StreamSendError::Closed(chunk));
        }
        if chunk.len() > state.max_chunk_bytes {
            return Err(StreamSendError::TooLarge(chunk));
        }
        if state.chunks.len() >= state.chunk_capacity {
            return Err(StreamSendError::Full(chunk));
        }
        state.chunks.push_back(chunk);
        Ok(())
    }

    fn finish(&mut self) {
        let mut state = lock_stream(&self.state);
        if !state.cancelled {
            state.send_open = false;
        }
    }

    fn cancel(&mut self) {
        lock_stream(&self.state).cancel();
    }
}

impl Drop for MemorySendStream {
    fn drop(&mut self) {
        let mut state = lock_stream(&self.state);
        if state.send_open && !state.cancelled {
            state.cancel();
        }
    }
}

struct MemoryReceiveStream {
    state: Arc<Mutex<MemoryStreamState>>,
}

impl TransportReceiveStream for MemoryReceiveStream {
    fn try_receive_chunk(&mut self) -> Result<Vec<u8>, StreamReceiveError> {
        let mut state = lock_stream(&self.state);
        if state.cancelled {
            return Err(StreamReceiveError::Cancelled);
        }
        if let Some(chunk) = state.chunks.pop_front() {
            return Ok(chunk);
        }
        if !state.send_open {
            return Err(StreamReceiveError::Finished);
        }
        Err(StreamReceiveError::Empty)
    }

    fn cancel(&mut self) {
        lock_stream(&self.state).cancel();
    }
}

impl Drop for MemoryReceiveStream {
    fn drop(&mut self) {
        let mut state = lock_stream(&self.state);
        if !state.reclaimable() {
            state.cancel();
        }
    }
}

#[derive(Clone)]
pub struct MemoryTransportFaults {
    state: Arc<Mutex<MemoryState>>,
}

impl MemoryTransportFaults {
    pub fn disconnect_now(&self) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .close_all();
    }

    pub fn close_outbound(&self, side: MemorySide) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !state.closed {
            state.close_outbound(side);
        }
    }
}

pub struct MemoryTransportPair {
    endpoint_a: MemoryTransportEndpoint,
    endpoint_b: MemoryTransportEndpoint,
    faults: MemoryTransportFaults,
}

impl MemoryTransportPair {
    pub fn new(control_capacity: NonZeroUsize, binding: [u8; 32]) -> Self {
        Self::with_config(
            MemoryTransportConfig::new(
                control_capacity,
                NonZeroUsize::new(DEFAULT_STREAM_CAPACITY)
                    .expect("default stream capacity is nonzero"),
                NonZeroUsize::new(DEFAULT_CHUNK_CAPACITY)
                    .expect("default chunk capacity is nonzero"),
                NonZeroUsize::new(DEFAULT_MAX_CHUNK_BYTES).expect("default chunk size is nonzero"),
            ),
            binding,
        )
    }

    pub fn with_config(config: MemoryTransportConfig, binding: [u8; 32]) -> Self {
        let state = Arc::new(Mutex::new(MemoryState::new(config)));
        let channel_binding = ChannelBinding::new(CHANNEL_BINDING_PROFILE, binding.to_vec());
        let endpoint_a = MemoryTransportEndpoint::new(
            MemorySide::A,
            Arc::clone(&state),
            channel_binding.clone(),
        );
        let endpoint_b =
            MemoryTransportEndpoint::new(MemorySide::B, Arc::clone(&state), channel_binding);
        let faults = MemoryTransportFaults { state };

        Self {
            endpoint_a,
            endpoint_b,
            faults,
        }
    }

    pub const fn endpoints(&self) -> (&MemoryTransportEndpoint, &MemoryTransportEndpoint) {
        (&self.endpoint_a, &self.endpoint_b)
    }

    pub const fn faults(&self) -> &MemoryTransportFaults {
        &self.faults
    }
}

fn lock_stream(state: &Arc<Mutex<MemoryStreamState>>) -> MutexGuard<'_, MemoryStreamState> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
