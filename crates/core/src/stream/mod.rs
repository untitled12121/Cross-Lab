use core::fmt;
use std::num::NonZeroUsize;

use crosslab_policy::{
    AuthorizedOperation, OperationError, OperationId, OperationState, OperationUseContext,
    UsePolicy,
};
use crosslab_protocol::{DataStreamOpen, StreamDirection, StreamId};

use crate::session::{LogicalSession, SessionState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmittedStream {
    stream_id: StreamId,
    operation_id: OperationId,
    stream_index: u32,
}

impl AdmittedStream {
    pub const fn stream_id(&self) -> StreamId {
        self.stream_id
    }

    pub const fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    pub const fn stream_index(&self) -> u32 {
        self.stream_index
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamAdmissionError {
    InactiveSession,
    InvalidSession,
    CapabilityNotNegotiated,
    OperationNotFound,
    DuplicateOperation,
    DuplicateStreamId,
    DuplicateStreamIndex,
    InvalidStreamIndex,
    StreamNotFound,
    ResourceLimit,
    Operation(OperationError),
}

impl fmt::Display for StreamAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InactiveSession => "logical session is not active",
            Self::InvalidSession => "data stream belongs to a different logical session",
            Self::CapabilityNotNegotiated => "data stream capability/version is not negotiated",
            Self::OperationNotFound => "authorized operation is not registered",
            Self::DuplicateOperation => "authorized operation is already registered",
            Self::DuplicateStreamId => "data stream identifier is already active",
            Self::DuplicateStreamIndex => "operation stream index is already used",
            Self::InvalidStreamIndex => "operation stream index is outside its use policy",
            Self::StreamNotFound => "admitted data stream is not active",
            Self::ResourceLimit => "stream admission state capacity is exhausted",
            Self::Operation(_) => "authorized operation rejected the data stream",
        })
    }
}

impl std::error::Error for StreamAdmissionError {}

impl From<OperationError> for StreamAdmissionError {
    fn from(error: OperationError) -> Self {
        Self::Operation(error)
    }
}

struct RegisteredOperation {
    operation: AuthorizedOperation,
    used_indices: Vec<u32>,
}

pub struct StreamAdmission {
    capacity: usize,
    operations: Vec<RegisteredOperation>,
    active_streams: Vec<AdmittedStream>,
}

impl StreamAdmission {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            capacity: capacity.get(),
            operations: Vec::with_capacity(capacity.get()),
            active_streams: Vec::with_capacity(capacity.get()),
        }
    }

    pub fn register_operation(
        &mut self,
        operation: AuthorizedOperation,
    ) -> Result<(), StreamAdmissionError> {
        if self
            .operations
            .iter()
            .any(|registered| registered.operation.id() == operation.id())
        {
            return Err(StreamAdmissionError::DuplicateOperation);
        }
        if self.operations.len() >= self.capacity || self.active_streams.len() >= self.capacity {
            return Err(StreamAdmissionError::ResourceLimit);
        }

        self.operations.push(RegisteredOperation {
            operation,
            used_indices: Vec::new(),
        });
        Ok(())
    }

    pub fn admit_inbound(
        &mut self,
        session: &LogicalSession,
        open: &DataStreamOpen,
        now: u64,
        current_trust_revision: u64,
        current_policy_revision: u64,
    ) -> Result<AdmittedStream, StreamAdmissionError> {
        if session.state() != SessionState::Active {
            return Err(StreamAdmissionError::InactiveSession);
        }
        let context = session
            .context()
            .ok_or(StreamAdmissionError::InactiveSession)?;
        if open.session_id() != context.session_id() {
            return Err(StreamAdmissionError::InvalidSession);
        }
        if !context.negotiated_capabilities().iter().any(|capability| {
            capability.capability_id() == open.capability_id()
                && capability.version() == open.capability_version()
        }) {
            return Err(StreamAdmissionError::CapabilityNotNegotiated);
        }
        if self
            .active_streams
            .iter()
            .any(|stream| stream.stream_id == open.stream_id())
        {
            return Err(StreamAdmissionError::DuplicateStreamId);
        }

        let operation_position = self
            .operations
            .iter()
            .position(|registered| registered.operation.id() == open.operation_id())
            .ok_or(StreamAdmissionError::OperationNotFound)?;
        let registered = &self.operations[operation_position];
        if !valid_stream_index(registered.operation.use_policy(), open.stream_index()) {
            return Err(StreamAdmissionError::InvalidStreamIndex);
        }
        if registered.used_indices.contains(&open.stream_index()) {
            return Err(StreamAdmissionError::DuplicateStreamIndex);
        }
        if registered.used_indices.len() >= self.capacity
            || self.active_streams.len() >= self.capacity
        {
            return Err(StreamAdmissionError::ResourceLimit);
        }

        let use_context = match open.direction() {
            StreamDirection::SourceToDestination => OperationUseContext::new(
                context.peer_device_id(),
                context.local_device_id(),
                context.session_id(),
                open.capability_id().clone(),
                open.capability_version(),
                open.operation_name().clone(),
            ),
            StreamDirection::DestinationToSource => OperationUseContext::new(
                context.local_device_id(),
                context.peer_device_id(),
                context.session_id(),
                open.capability_id().clone(),
                open.capability_version(),
                open.operation_name().clone(),
            ),
        };

        let reservation = self.operations[operation_position]
            .operation
            .reserve_stream_use(
                &use_context,
                now,
                current_trust_revision,
                current_policy_revision,
            );
        if let Err(error) = reservation {
            if self.operations[operation_position].operation.state() != OperationState::Active {
                self.operations.remove(operation_position);
            }
            return Err(error.into());
        }
        self.operations[operation_position]
            .used_indices
            .push(open.stream_index());

        let admitted = AdmittedStream {
            stream_id: open.stream_id(),
            operation_id: open.operation_id(),
            stream_index: open.stream_index(),
        };
        self.active_streams.push(admitted);
        Ok(admitted)
    }

    pub fn finish_stream(&mut self, stream_id: StreamId) -> Result<(), StreamAdmissionError> {
        self.finish_stream_inner(stream_id)
    }

    pub fn cancel_stream(&mut self, stream_id: StreamId) -> Result<(), StreamAdmissionError> {
        self.finish_stream_inner(stream_id)
    }

    pub fn cancel_all(&mut self) {
        self.active_streams.clear();
        for registered in &mut self.operations {
            registered.operation.cancel();
        }
        self.operations.clear();
    }

    pub fn active_stream_count(&self) -> usize {
        self.active_streams.len()
    }

    fn finish_stream_inner(&mut self, stream_id: StreamId) -> Result<(), StreamAdmissionError> {
        let position = self
            .active_streams
            .iter()
            .position(|stream| stream.stream_id == stream_id)
            .ok_or(StreamAdmissionError::StreamNotFound)?;
        let admitted = self.active_streams.remove(position);
        self.reap_operation_if_terminal(admitted.operation_id);
        Ok(())
    }

    fn reap_operation_if_terminal(&mut self, operation_id: OperationId) {
        if self
            .active_streams
            .iter()
            .any(|stream| stream.operation_id == operation_id)
        {
            return;
        }
        let Some(position) = self
            .operations
            .iter()
            .position(|registered| registered.operation.id() == operation_id)
        else {
            return;
        };
        if self.operations[position].operation.stream_budget_exhausted() {
            self.operations[position].operation.consume();
        }
        if self.operations[position].operation.state() != OperationState::Active {
            self.operations.remove(position);
        }
    }
}

const fn valid_stream_index(use_policy: UsePolicy, stream_index: u32) -> bool {
    match use_policy {
        UsePolicy::SingleAction => true,
        UsePolicy::SingleStream => stream_index == 0,
        UsePolicy::MultiStream { max_streams } => stream_index < max_streams.get(),
    }
}
