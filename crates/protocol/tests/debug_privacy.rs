use crosslab_crypto::Signature;
use crosslab_policy::{CapabilityId, CapabilityVersion, OperationName, SessionId};
use crosslab_protocol::{
    ControlEnvelope, ControlRequest, ControlResponse, ControlResponseResult, EnvelopeBody, Event,
    EventId, EventType, PairingConfirmation, PairingRole, ProtocolVersion, RequestId, RetryClass,
    SessionAuthProofMessage, SessionAuthRole,
};

fn request(body: Vec<u8>) -> ControlRequest {
    ControlRequest::new(
        RequestId::from_bytes([0x91; 16]),
        CapabilityId::parse("clipboard.write").unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("set").unwrap(),
        RetryClass::NonRetryable,
        body,
    )
}

#[test]
fn control_payloads_are_redacted_from_debug_output() {
    let payload = vec![201, 202, 203, 204];
    let request = request(payload.clone());
    let response = ControlResponse::new(
        RequestId::from_bytes([0x92; 16]),
        ControlResponseResult::Success(payload.clone()),
    );
    let event = Event::capability(
        EventId::from_bytes([0x93; 16]),
        CapabilityId::parse("clipboard.write").unwrap(),
        EventType::parse("clipboard.changed").unwrap(),
        payload.clone(),
    )
    .unwrap();
    let envelope = ControlEnvelope::new(
        ProtocolVersion::new(1, 0),
        SessionId::from_bytes([0x94; 32]),
        0,
        EnvelopeBody::ControlRequest(request.clone()),
    );

    for debug in [
        format!("{request:?}"),
        format!("{response:?}"),
        format!("{event:?}"),
        format!("{envelope:?}"),
    ] {
        assert!(!debug.contains("201, 202, 203, 204"), "{debug}");
    }
}

#[test]
fn pairing_and_session_auth_proofs_are_redacted_from_debug_output() {
    let confirmation = PairingConfirmation::new(PairingRole::Joiner, [0x95; 16], [211; 32]);
    let proof = SessionAuthProofMessage::new(
        SessionAuthRole::Initiator,
        [212; 32],
        Signature::from_bytes([213; 64]),
    );

    let pairing_debug = format!("{confirmation:?}");
    let proof_debug = format!("{proof:?}");

    assert!(!pairing_debug.contains("211, 211"), "{pairing_debug}");
    assert!(!proof_debug.contains("212, 212"), "{proof_debug}");
    assert!(!proof_debug.contains("213, 213"), "{proof_debug}");
}
