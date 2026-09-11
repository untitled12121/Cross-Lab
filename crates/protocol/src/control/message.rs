use crosslab_policy::{CapabilityId, CapabilityVersion, OperationName};

use super::{ProtocolFailure, RequestId, RetryClass};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRequest {
    request_id: RequestId,
    capability_id: CapabilityId,
    capability_version: CapabilityVersion,
    operation_name: OperationName,
    retry_class: RetryClass,
    body: Vec<u8>,
}

impl ControlRequest {
    pub fn new(
        request_id: RequestId,
        capability_id: CapabilityId,
        capability_version: CapabilityVersion,
        operation_name: OperationName,
        retry_class: RetryClass,
        body: Vec<u8>,
    ) -> Self {
        Self {
            request_id,
            capability_id,
            capability_version,
            operation_name,
            retry_class,
            body,
        }
    }

    pub const fn request_id(&self) -> RequestId {
        self.request_id
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

    pub const fn retry_class(&self) -> RetryClass {
        self.retry_class
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlResponseResult {
    Success(Vec<u8>),
    Error(ProtocolFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlResponse {
    request_id: RequestId,
    result: ControlResponseResult,
}

impl ControlResponse {
    pub fn new(request_id: RequestId, result: ControlResponseResult) -> Self {
        Self { request_id, result }
    }

    pub const fn request_id(&self) -> RequestId {
        self.request_id
    }

    pub const fn result(&self) -> &ControlResponseResult {
        &self.result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelRequest {
    request_id: RequestId,
}

impl CancelRequest {
    pub const fn new(request_id: RequestId) -> Self {
        Self { request_id }
    }

    pub const fn request_id(&self) -> RequestId {
        self.request_id
    }
}
