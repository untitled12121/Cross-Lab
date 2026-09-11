use crosslab_policy::SessionId;
use crosslab_protocol::{
    ControlEnvelope, EnvelopeBody, ProtocolDiagnostic, ProtocolVersion, ProtocolWireError,
    SessionClose, SessionCloseReason, decode_control_envelope, encode_control_envelope, wire,
};

#[test]
fn session_close_round_trips_through_the_control_envelope() {
    let close = SessionClose::new(
        SessionCloseReason::LocalRequest,
        Some(ProtocolDiagnostic::new("leaving session").unwrap()),
    );
    let expected = ControlEnvelope::new(
        ProtocolVersion::new(1, 0),
        SessionId::from_bytes([4; 32]),
        8,
        EnvelopeBody::SessionClose(close),
    );

    let encoded = encode_control_envelope(&expected).unwrap();
    assert_eq!(decode_control_envelope(&encoded).unwrap(), expected);
}

#[test]
fn session_close_wire_conversion_rejects_unspecified_and_unknown_reasons() {
    for reason in [0, 7] {
        let wire = wire::v1::SessionCloseV1 {
            reason,
            diagnostic: None,
        };
        assert_eq!(
            SessionClose::try_from(wire).unwrap_err(),
            ProtocolWireError::InvalidSessionCloseReason(reason)
        );
    }
}

#[test]
fn session_close_wire_conversion_rejects_oversized_diagnostic() {
    let wire = wire::v1::SessionCloseV1 {
        reason: wire::v1::SessionCloseReasonV1::Shutdown as i32,
        diagnostic: Some("a".repeat(513)),
    };

    assert_eq!(
        SessionClose::try_from(wire).unwrap_err(),
        ProtocolWireError::InvalidDiagnostic
    );
}
