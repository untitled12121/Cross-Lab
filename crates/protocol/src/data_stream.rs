use crosslab_policy::{
    CapabilityId, CapabilityVersion, OperationId, OperationName, SessionId,
};

use crate::StreamId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamDirection {
    SourceToDestination,
    DestinationToSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataStreamOpen {
    session_id: SessionId,
    stream_id: StreamId,
    operation_id: OperationId,
    capability_id: CapabilityId,
    capability_version: CapabilityVersion,
    operation_name: OperationName,
    direction: StreamDirection,
    stream_index: u32,
}

impl DataStreamOpen {
    pub fn new(
        session_id: SessionId,
        stream_id: StreamId,
        operation_id: OperationId,
        capability_id: CapabilityId,
        capability_version: CapabilityVersion,
        operation_name: OperationName,
        direction: StreamDirection,
        stream_index: u32,
    ) -> Self {
        Self {
            session_id,
            stream_id,
            operation_id,
            capability_id,
            capability_version,
            operation_name,
            direction,
            stream_index,
        }
    }

    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub const fn stream_id(&self) -> StreamId {
        self.stream_id
    }

    pub const fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    pub fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
    }

    pub const fn capability_version(&self) -> CapabilityVersion {
        self.capability_version
    }

    pub fn operation_name(&self) -> &OperationName {
        &self.operation_name
    }

    pub const fn direction(&self) -> StreamDirection {
        self.direction
    }

    pub const fn stream_index(&self) -> u32 {
        self.stream_index
    }
}
