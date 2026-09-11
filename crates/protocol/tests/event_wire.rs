use crosslab_policy::{CapabilityId, SessionId};
use crosslab_protocol::{
    ControlEnvelope, EnvelopeBody, Event, EventId, EventType, ProtocolVersion, ProtocolWireError,
    decode_control_envelope, encode_control_envelope, wire,
};

#[test]
fn capability_event_round_trips_through_the_control_envelope() {
    let event = Event::capability(
        EventId::from_bytes([1; 16]),
        CapabilityId::parse("clipboard.write").unwrap(),
        EventType::parse("clipboard.changed").unwrap(),
        b"value".to_vec(),
    )
    .unwrap();
    let expected = ControlEnvelope::new(
        ProtocolVersion::new(1, 0),
        SessionId::from_bytes([2; 32]),
        4,
        EnvelopeBody::Event(event),
    );

    let encoded = encode_control_envelope(&expected).unwrap();
    assert_eq!(decode_control_envelope(&encoded).unwrap(), expected);
}

#[test]
fn system_event_round_trips_without_a_capability_id() {
    let event = Event::system(
        EventId::from_bytes([3; 16]),
        EventType::parse("crosslab.system.trust-revoked").unwrap(),
        Vec::new(),
    )
    .unwrap();
    let wire = wire::v1::EventV1::from(&event);

    assert_eq!(wire.capability_id, None);
    assert_eq!(Event::try_from(wire).unwrap(), event);
}

#[test]
fn event_wire_conversion_rejects_invalid_ids_types_and_scope() {
    let invalid_id = wire::v1::EventV1 {
        event_id: vec![1; 15],
        capability_id: Some("clipboard.write".to_owned()),
        event_type: "clipboard.changed".to_owned(),
        body: Vec::new(),
    };
    assert_eq!(
        Event::try_from(invalid_id).unwrap_err(),
        ProtocolWireError::InvalidEventIdLength(15)
    );

    let invalid_type = wire::v1::EventV1 {
        event_id: vec![1; 16],
        capability_id: Some("clipboard.write".to_owned()),
        event_type: "Clipboard.changed".to_owned(),
        body: Vec::new(),
    };
    assert_eq!(
        Event::try_from(invalid_type).unwrap_err(),
        ProtocolWireError::InvalidEventType
    );

    let unscoped_capability_event = wire::v1::EventV1 {
        event_id: vec![1; 16],
        capability_id: None,
        event_type: "clipboard.changed".to_owned(),
        body: Vec::new(),
    };
    assert_eq!(
        Event::try_from(unscoped_capability_event).unwrap_err(),
        ProtocolWireError::InvalidEventScope
    );

    let system_type_with_capability = wire::v1::EventV1 {
        event_id: vec![1; 16],
        capability_id: Some("clipboard.write".to_owned()),
        event_type: "crosslab.system.trust-revoked".to_owned(),
        body: Vec::new(),
    };
    assert_eq!(
        Event::try_from(system_type_with_capability).unwrap_err(),
        ProtocolWireError::InvalidEventScope
    );
}
