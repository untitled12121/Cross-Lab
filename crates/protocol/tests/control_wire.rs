use crosslab_policy::{CapabilityId, CapabilityVersion, OperationName};
use crosslab_protocol::{
    CancelRequest, ControlRequest, ControlResponse, ControlResponseResult, ProtocolDiagnostic,
    ProtocolErrorCode, ProtocolFailure, ProtocolWireError, RequestId, RetryClass, wire,
};

fn request() -> ControlRequest {
    ControlRequest::new(
        RequestId::from_bytes([1; 16]),
        CapabilityId::parse("clipboard.write").unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("set").unwrap(),
        RetryClass::Idempotent,
        b"value".to_vec(),
    )
}

fn wire_request() -> wire::v1::ControlRequestV1 {
    wire::v1::ControlRequestV1 {
        request_id: vec![1; 16],
        capability_id: "clipboard.write".into(),
        capability_version: Some(wire::v1::CapabilityVersionV1 { major: 1, minor: 0 }),
        operation_name: "set".into(),
        retry_class: wire::v1::RetryClassV1::Idempotent as i32,
        body: b"value".to_vec(),
    }
}

#[test]
fn control_request_round_trips_through_strict_wire_conversion() {
    let expected = request();
    let wire = wire::v1::ControlRequestV1::from(&expected);
    let decoded = ControlRequest::try_from(wire).unwrap();

    assert_eq!(decoded, expected);
}

#[test]
fn control_request_rejects_invalid_request_id_length_and_retry_class() {
    let mut invalid_id = wire_request();
    invalid_id.request_id.pop();
    assert_eq!(
        ControlRequest::try_from(invalid_id).unwrap_err(),
        ProtocolWireError::InvalidRequestIdLength(15)
    );

    let mut invalid_retry = wire_request();
    invalid_retry.retry_class = 7;
    assert_eq!(
        ControlRequest::try_from(invalid_retry).unwrap_err(),
        ProtocolWireError::InvalidRetryClass(7)
    );
}

#[test]
fn control_response_round_trips_success_and_typed_error() {
    let success = ControlResponse::new(
        RequestId::from_bytes([2; 16]),
        ControlResponseResult::Success(b"ok".to_vec()),
    );
    assert_eq!(
        ControlResponse::try_from(wire::v1::ControlResponseV1::from(&success)).unwrap(),
        success
    );

    let failure = ProtocolFailure::new(
        ProtocolErrorCode::AuthorizationDenied,
        Some(ProtocolDiagnostic::new("denied").unwrap()),
    );
    let error = ControlResponse::new(
        RequestId::from_bytes([3; 16]),
        ControlResponseResult::Error(failure),
    );
    assert_eq!(
        ControlResponse::try_from(wire::v1::ControlResponseV1::from(&error)).unwrap(),
        error
    );
}

#[test]
fn control_response_rejects_missing_result_and_unknown_error_code() {
    let missing = wire::v1::ControlResponseV1 {
        request_id: vec![2; 16],
        result: None,
    };
    assert_eq!(
        ControlResponse::try_from(missing).unwrap_err(),
        ProtocolWireError::MissingControlResponseResult
    );

    let unknown_error = wire::v1::ControlResponseV1 {
        request_id: vec![2; 16],
        result: Some(wire::v1::control_response_v1::Result::Error(
            wire::v1::ProtocolErrorV1 {
                code: 99,
                diagnostic: None,
            },
        )),
    };
    assert_eq!(
        ControlResponse::try_from(unknown_error).unwrap_err(),
        ProtocolWireError::InvalidProtocolErrorCode(99)
    );
}

#[test]
fn cancel_request_round_trips_and_validates_request_id_length() {
    let expected = CancelRequest::new(RequestId::from_bytes([4; 16]));
    assert_eq!(
        CancelRequest::try_from(wire::v1::CancelRequestV1::from(&expected)).unwrap(),
        expected
    );

    assert_eq!(
        CancelRequest::try_from(wire::v1::CancelRequestV1 {
            request_id: vec![4; 17],
        })
        .unwrap_err(),
        ProtocolWireError::InvalidRequestIdLength(17)
    );
}
