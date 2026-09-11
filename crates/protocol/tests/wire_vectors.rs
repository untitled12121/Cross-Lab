use crosslab_policy::{CapabilityId, CapabilityVersion, OperationId, OperationName, SessionId};
use crosslab_protocol::{
    CancelRequest, ControlEnvelope, DataStreamOpen, EnvelopeBody, Event, EventId, EventType,
    PairingBootstrapMessage, PairingConfirmation, PairingRole, ProtocolVersion, RequestId,
    SessionClose, SessionCloseReason, StreamDirection, StreamId, encode_control_envelope,
    encode_data_stream_open, encode_pairing_bootstrap,
};

#[test]
fn data_stream_open_v1_matches_golden_frame_bytes() {
    let header = DataStreamOpen::new(
        SessionId::from_bytes([0x01; 32]),
        StreamId::from_bytes([0x02; 16]),
        OperationId::from_bytes([0x03; 32]),
        CapabilityId::parse("clipboard.write").unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("set").unwrap(),
        StreamDirection::SourceToDestination,
        1,
    );

    let expected = vec![
        0x00, 0x00, 0x00, 0x74, 0x0a, 0x20, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x12, 0x10, 0x02, 0x02, 0x02, 0x02, 0x02,
        0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x1a, 0x20, 0x03, 0x03,
        0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
        0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
        0x22, 0x0f, 0x63, 0x6c, 0x69, 0x70, 0x62, 0x6f, 0x61, 0x72, 0x64, 0x2e, 0x77, 0x72, 0x69,
        0x74, 0x65, 0x2a, 0x02, 0x08, 0x01, 0x32, 0x03, 0x73, 0x65, 0x74, 0x38, 0x01, 0x40, 0x01,
    ];

    assert_eq!(encode_data_stream_open(&header).unwrap(), expected);
}

#[test]
fn control_envelope_v1_matches_golden_frame_bytes() {
    let envelope = ControlEnvelope::new(
        ProtocolVersion::new(1, 0),
        SessionId::from_bytes([0x04; 32]),
        7,
        EnvelopeBody::CancelRequest(CancelRequest::new(RequestId::from_bytes([0x05; 16]))),
    );

    let mut expected = vec![0x00, 0x00, 0x00, 0x3a, 0x08, 0x01, 0x1a, 0x20];
    expected.extend_from_slice(&[0x04; 32]);
    expected.extend_from_slice(&[0x20, 0x07, 0x4a, 0x12, 0x0a, 0x10]);
    expected.extend_from_slice(&[0x05; 16]);

    assert_eq!(encode_control_envelope(&envelope).unwrap(), expected);
}

#[test]
fn system_event_v1_matches_golden_frame_bytes() {
    let event = Event::system(
        EventId::from_bytes([0x07; 16]),
        EventType::parse("crosslab.system.shutdown").unwrap(),
        Vec::new(),
    )
    .unwrap();
    let envelope = ControlEnvelope::new(
        ProtocolVersion::new(1, 0),
        SessionId::from_bytes([0x06; 32]),
        9,
        EnvelopeBody::Event(event),
    );

    let mut expected = vec![0x00, 0x00, 0x00, 0x54, 0x08, 0x01, 0x1a, 0x20];
    expected.extend_from_slice(&[0x06; 32]);
    expected.extend_from_slice(&[0x20, 0x09, 0x42, 0x2c, 0x0a, 0x10]);
    expected.extend_from_slice(&[0x07; 16]);
    expected.extend_from_slice(&[0x1a, 0x18]);
    expected.extend_from_slice(b"crosslab.system.shutdown");

    assert_eq!(encode_control_envelope(&envelope).unwrap(), expected);
}

#[test]
fn session_close_v1_matches_golden_frame_bytes() {
    let envelope = ControlEnvelope::new(
        ProtocolVersion::new(1, 0),
        SessionId::from_bytes([0x08; 32]),
        10,
        EnvelopeBody::SessionClose(SessionClose::new(SessionCloseReason::Shutdown, None)),
    );

    let mut expected = vec![0x00, 0x00, 0x00, 0x2a, 0x08, 0x01, 0x1a, 0x20];
    expected.extend_from_slice(&[0x08; 32]);
    expected.extend_from_slice(&[0x20, 0x0a, 0x5a, 0x02, 0x08, 0x06]);

    assert_eq!(encode_control_envelope(&envelope).unwrap(), expected);
}

#[test]
fn pairing_confirmation_v1_matches_golden_frame_bytes() {
    let confirmation = PairingBootstrapMessage::Confirmation(PairingConfirmation::new(
        PairingRole::Joiner,
        [0x11; 16],
        [0x22; 32],
    ));

    let mut expected = vec![0x00, 0x00, 0x00, 0x3a, 0x08, 0x01, 0x1a, 0x36, 0x08, 0x02, 0x12, 0x10];
    expected.extend_from_slice(&[0x11; 16]);
    expected.extend_from_slice(&[0x1a, 0x20]);
    expected.extend_from_slice(&[0x22; 32]);

    assert_eq!(encode_pairing_bootstrap(&confirmation).unwrap(), expected);
}
