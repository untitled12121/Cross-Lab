use crosslab_core::ConnectionMetadata;

#[test]
fn connection_metadata_debug_redacts_endpoint_descriptions() {
    let metadata = ConnectionMetadata::new(
        Some("192.0.2.10:4242".to_owned()),
        Some("198.51.100.20:8484".to_owned()),
        Some(true),
    );

    let debug = format!("{metadata:?}");
    assert!(!debug.contains("192.0.2.10:4242"));
    assert!(!debug.contains("198.51.100.20:8484"));
    assert!(debug.contains("[REDACTED]"));
    assert!(debug.contains("metered"));
    assert!(debug.contains("true"));
}
