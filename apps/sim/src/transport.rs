use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use crosslab_core::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, TransportConnection,
    TransportSecurityClass,
};

const CHANNEL_BINDING_PROFILE: &str = "in-process-test";

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

struct DirectionState {
    queue: VecDeque<Vec<u8>>,
    capacity: usize,
    open: bool,
}

impl DirectionState {
    fn new(capacity: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity),
            capacity,
            open: true,
        }
    }
}

struct MemoryState {
    a_to_b: DirectionState,
    b_to_a: DirectionState,
    closed: bool,
}

impl MemoryState {
    fn new(capacity: usize) -> Self {
        Self {
            a_to_b: DirectionState::new(capacity),
            b_to_a: DirectionState::new(capacity),
            closed: false,
        }
    }

    fn outbound_mut(&mut self, side: MemorySide) -> &mut DirectionState {
        match side {
            MemorySide::A => &mut self.a_to_b,
            MemorySide::B => &mut self.b_to_a,
        }
    }

    fn inbound_mut(&mut self, side: MemorySide) -> &mut DirectionState {
        self.outbound_mut(side.opposite())
    }

    fn close_all(&mut self) {
        self.closed = true;
        self.a_to_b.open = false;
        self.b_to_a.open = false;
        self.a_to_b.queue.clear();
        self.b_to_a.queue.clear();
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

    fn lock_state(&self) -> std::sync::MutexGuard<'_, MemoryState> {
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

        let outbound = state.outbound_mut(self.side);
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

        let inbound = state.inbound_mut(self.side);
        if let Some(frame) = inbound.queue.pop_front() {
            return Ok(frame);
        }
        if !inbound.open {
            return Err(ControlReceiveError::Closed);
        }

        Err(ControlReceiveError::Empty)
    }

    fn close(&self) {
        self.lock_state().close_all();
    }

    fn is_closed(&self) -> bool {
        self.lock_state().closed
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
            state.outbound_mut(side).open = false;
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
        let state = Arc::new(Mutex::new(MemoryState::new(control_capacity.get())));
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
