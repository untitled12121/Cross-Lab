use crosslab_policy::CapabilityId;
use crosslab_protocol::{Event, EventId, EventScope, EventType};

#[test]
fn event_type_accepts_only_canonical_namespaced_ascii() {
    let event_type = EventType::parse("clipboard.changed").unwrap();
    assert_eq!(event_type.as_str(), "clipboard.changed");

    for invalid in [
        "",
        "changed",
        "Clipboard.changed",
        "clipboard..changed",
        "clipboard.changed_",
        ".clipboard.changed",
        "clipboard.changed.",
    ] {
        assert!(EventType::parse(invalid).is_err(), "accepted {invalid:?}");
    }
}

#[test]
fn capability_and_system_event_scopes_are_explicit() {
    let capability = CapabilityId::parse("clipboard.write").unwrap();
    let capability_type = EventType::parse("clipboard.changed").unwrap();
    let event = Event::capability(
        EventId::from_bytes([1; 16]),
        capability.clone(),
        capability_type,
        b"value".to_vec(),
    )
    .unwrap();

    assert!(matches!(
        event.scope(),
        EventScope::Capability(value) if value == &capability
    ));
    assert_eq!(event.body(), b"value");

    let system_type = EventType::parse("crosslab.system.trust-revoked").unwrap();
    let system = Event::system(
        EventId::from_bytes([2; 16]),
        system_type.clone(),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(system.scope(), &EventScope::System);

    assert!(
        Event::system(
            EventId::from_bytes([3; 16]),
            EventType::parse("clipboard.changed").unwrap(),
            Vec::new(),
        )
        .is_err()
    );
    assert!(
        Event::capability(
            EventId::from_bytes([4; 16]),
            capability,
            system_type,
            Vec::new(),
        )
        .is_err()
    );
}
