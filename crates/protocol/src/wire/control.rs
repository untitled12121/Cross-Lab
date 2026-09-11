use crosslab_policy::{CapabilityId, OperationName};

use crate::{
    CancelRequest, ControlRequest, ControlResponse, ControlResponseResult, ProtocolDiagnostic,
    ProtocolErrorCode, ProtocolFailure, RequestId, RetryClass,
};

use super::{
    codec::{ProtocolWireError, capability_version_from_wire, copy_16},
    v1::{
        CancelRequestV1, CapabilityVersionV1, ControlRequestV1, ControlResponseV1, ProtocolErrorV1,
        RetryClassV1, control_response_v1,
    },
};

impl From<&ControlRequest> for ControlRequestV1 {
    fn from(request: &ControlRequest) -> Self {
        Self {
            request_id: request.request_id().to_bytes().to_vec(),
            capability_id: request.capability_id().as_str().to_owned(),
            capability_version: Some(CapabilityVersionV1 {
                major: request.capability_version().major().into(),
                minor: request.capability_version().minor().into(),
            }),
            operation_name: request.operation_name().as_str().to_owned(),
            retry_class: match request.retry_class() {
                RetryClass::NonRetryable => RetryClassV1::NonRetryable as i32,
                RetryClass::Idempotent => RetryClassV1::Idempotent as i32,
            },
            body: request.body().to_vec(),
        }
    }
}

impl TryFrom<ControlRequestV1> for ControlRequest {
    type Error = ProtocolWireError;

    fn try_from(wire: ControlRequestV1) -> Result<Self, Self::Error> {
        let request_id = RequestId::from_bytes(copy_16(
            wire.request_id,
            ProtocolWireError::InvalidRequestIdLength,
        )?);
        let capability_id = CapabilityId::parse(&wire.capability_id)
            .map_err(|_| ProtocolWireError::InvalidCapabilityId)?;
        let capability_version = wire
            .capability_version
            .ok_or(ProtocolWireError::MissingCapabilityVersion)
            .and_then(capability_version_from_wire)?;
        let operation_name = OperationName::parse(&wire.operation_name)
            .map_err(|_| ProtocolWireError::InvalidOperationName)?;
        let retry_class = match wire.retry_class {
            value if value == RetryClassV1::NonRetryable as i32 => RetryClass::NonRetryable,
            value if value == RetryClassV1::Idempotent as i32 => RetryClass::Idempotent,
            value => return Err(ProtocolWireError::InvalidRetryClass(value)),
        };

        Ok(Self::new(
            request_id,
            capability_id,
            capability_version,
            operation_name,
            retry_class,
            wire.body,
        ))
    }
}

impl From<&ControlResponse> for ControlResponseV1 {
    fn from(response: &ControlResponse) -> Self {
        let result = match response.result() {
            ControlResponseResult::Success(body) => {
                control_response_v1::Result::SuccessBody(body.clone())
            }
            ControlResponseResult::Error(error) => {
                control_response_v1::Result::Error(ProtocolErrorV1::from(error))
            }
        };
        Self {
            request_id: response.request_id().to_bytes().to_vec(),
            result: Some(result),
        }
    }
}

impl TryFrom<ControlResponseV1> for ControlResponse {
    type Error = ProtocolWireError;

    fn try_from(wire: ControlResponseV1) -> Result<Self, Self::Error> {
        let request_id = RequestId::from_bytes(copy_16(
            wire.request_id,
            ProtocolWireError::InvalidRequestIdLength,
        )?);
        let result = match wire
            .result
            .ok_or(ProtocolWireError::MissingControlResponseResult)?
        {
            control_response_v1::Result::SuccessBody(body) => ControlResponseResult::Success(body),
            control_response_v1::Result::Error(error) => {
                ControlResponseResult::Error(ProtocolFailure::try_from(error)?)
            }
        };
        Ok(Self::new(request_id, result))
    }
}

impl From<&ProtocolFailure> for ProtocolErrorV1 {
    fn from(failure: &ProtocolFailure) -> Self {
        Self {
            code: failure.code().code().into(),
            diagnostic: failure
                .diagnostic()
                .map(|diagnostic| diagnostic.as_str().to_owned()),
        }
    }
}

impl TryFrom<ProtocolErrorV1> for ProtocolFailure {
    type Error = ProtocolWireError;

    fn try_from(wire: ProtocolErrorV1) -> Result<Self, Self::Error> {
        let code = u16::try_from(wire.code)
            .ok()
            .and_then(|code| ProtocolErrorCode::from_code(code).ok())
            .ok_or(ProtocolWireError::InvalidProtocolErrorCode(wire.code))?;
        let diagnostic = wire
            .diagnostic
            .map(|value| {
                ProtocolDiagnostic::new(&value).map_err(|_| ProtocolWireError::InvalidDiagnostic)
            })
            .transpose()?;
        Ok(Self::new(code, diagnostic))
    }
}

impl From<&CancelRequest> for CancelRequestV1 {
    fn from(cancel: &CancelRequest) -> Self {
        Self {
            request_id: cancel.request_id().to_bytes().to_vec(),
        }
    }
}

impl TryFrom<CancelRequestV1> for CancelRequest {
    type Error = ProtocolWireError;

    fn try_from(wire: CancelRequestV1) -> Result<Self, Self::Error> {
        let request_id = RequestId::from_bytes(copy_16(
            wire.request_id,
            ProtocolWireError::InvalidRequestIdLength,
        )?);
        Ok(Self::new(request_id))
    }
}
