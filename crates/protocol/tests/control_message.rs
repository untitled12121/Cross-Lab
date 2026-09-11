use crosslab_policy::{CapabilityId, CapabilityVersion, OperationName};
use crosslab_protocol::{
    CancelRequest, ControlRequest, ControlResponse, ControlResponseResult, ProtocolDiagnostic,
    ProtocolErrorCode, ProtocolFailure, RequestId, RetryClass,
};

fn request() -> ControlRequest {
    ControlRequest::new(
        RequestId::from_bytes([1; 16]),
        CapabilityId::parse("clipboard.write").unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("set").unwrap(),
        RetryClass::NonRetryable,
        b"value".to_vec(),
    )
}

#[test]
fn control_request_preserves_correlation_and_operation_header() {
    let request = request();

    assert_eq!(request.request_id().to_bytes(), [1; 16]);
    assert_eq!(request.capability_id().as_str(), "clipboard.write");
    assert_eq!(request.capability_version(), CapabilityVersion::new(1, 0));
    assert_eq!(request.operation_name().as_str(), "set");
    assert_eq!(request.retry_class(), RetryClass::NonRetryable);
    assert_eq!(request.body(), b"value");
}

#[test]
fn control_response_preserves_request_correlation_and_typed_result() {
    let success = ControlResponse::new(
        RequestId::from_bytes([2; 16]),
        ControlResponseResult::Success(b"ok".to_vec()),
    );
    assert_eq!(success.request_id().to_bytes(), [2; 16]);
    assert_eq!(success.result(), &ControlResponseResult::Success(b"ok".to_vec()));

    let failure = ProtocolFailure::new(
        ProtocolErrorCode::AuthorizationDenied,
        Some(ProtocolDiagnostic::new("denied").unwrap()),
    );
    let error = ControlResponse::new(
        RequestId::from_bytes([3; 16]),
        ControlResponseResult::Error(failure.clone()),
    );
    assert_eq!(error.result(), &ControlResponseResult::Error(failure));
}

#[test]
fn cancel_request_references_only_the_request_id() {
    let cancel = CancelRequest::new(RequestId::from_bytes([4; 16]));
    assert_eq!(cancel.request_id().to_bytes(), [4; 16]);
}
