use crosslab_policy::{CapabilityId, CapabilityVersion, OperationName, SessionId};
use crosslab_protocol::{
    ControlEnvelope, ControlRequest, EnvelopeBody, ProtocolVersion, ProtocolWireError, RequestId,
    RetryClass, decode_control_envelope, encode_control_envelope, wire,
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
fn control_envelope_round_trips_with_session_sequence_and_body() {
    let expected = ControlEnvelope::new(
        ProtocolVersion::new(1, 0),
        SessionId::from_bytes([2; 32]),
        7,
        EnvelopeBody::ControlRequest(request()),
    );

    let encoded = encode_control_envelope(&expected).unwrap();
    let decoded = decode_control_envelope(&encoded).unwrap();

    assert_eq!(decoded, expected);
}

#[test]
fn wire_envelope_rejects_missing_body() {
    let wire = wire::v1::EnvelopeV1 {
        protocol_major: 1,
        protocol_minor: 0,
        session_id: vec![2; 32],
        message_seq: 0,
        body: None,
    };

    assert_eq!(
        ControlEnvelope::try_from(wire).unwrap_err(),
        ProtocolWireError::MissingEnvelopeBody
    );
}

#[test]
fn wire_envelope_rejects_invalid_session_and_protocol_version() {
    let invalid_session = wire::v1::EnvelopeV1 {
        protocol_major: 1,
        protocol_minor: 0,
        session_id: vec![2; 31],
        message_seq: 0,
        body: Some(wire::v1::envelope_v1::Body::ControlRequest(
            wire::v1::ControlRequestV1::from(&request()),
        )),
    };
    assert_eq!(
        ControlEnvelope::try_from(invalid_session).unwrap_err(),
        ProtocolWireError::InvalidSessionIdLength(31)
    );

    let invalid_version = wire::v1::EnvelopeV1 {
        protocol_major: u32::from(u16::MAX) + 1,
        protocol_minor: 0,
        session_id: vec![2; 32],
        message_seq: 0,
        body: Some(wire::v1::envelope_v1::Body::ControlRequest(
            wire::v1::ControlRequestV1::from(&request()),
        )),
    };
    assert_eq!(
        ControlEnvelope::try_from(invalid_version).unwrap_err(),
        ProtocolWireError::InvalidProtocolVersion
    );
}
