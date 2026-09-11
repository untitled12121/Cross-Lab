use crosslab_protocol::{
    EventId, ProtocolDiagnostic, ProtocolIdError, RequestId, RetryClass, RetryClassError, StreamId,
};

#[test]
fn protocol_ids_preserve_exact_128_bit_values() {
    let request = RequestId::from_bytes([0x11; 16]);
    let event = EventId::from_bytes([0x22; 16]);
    let stream = StreamId::from_bytes([0x33; 16]);

    assert_eq!(request.to_bytes(), [0x11; 16]);
    assert_eq!(event.to_bytes(), [0x22; 16]);
    assert_eq!(stream.to_bytes(), [0x33; 16]);
}

#[test]
fn protocol_ids_use_secure_random_generation_api() {
    let request = RequestId::generate();
    let event = EventId::generate();
    let stream = StreamId::generate();

    assert!(request.is_ok());
    assert!(event.is_ok());
    assert!(stream.is_ok());
    let _: Result<RequestId, ProtocolIdError> = request;
}

#[test]
fn retry_class_matches_protocol_v1_registry() {
    assert_eq!(RetryClass::NonRetryable.code(), 0);
    assert_eq!(RetryClass::Idempotent.code(), 1);
    assert_eq!(RetryClass::from_code(0).unwrap(), RetryClass::NonRetryable);
    assert_eq!(RetryClass::from_code(1).unwrap(), RetryClass::Idempotent);
    assert_eq!(
        RetryClass::from_code(2).unwrap_err(),
        RetryClassError::UnknownValue(2)
    );
}

#[test]
fn safe_protocol_diagnostic_is_limited_to_512_utf8_bytes() {
    let max = "a".repeat(512);
    let oversized = "a".repeat(513);

    assert_eq!(ProtocolDiagnostic::new(&max).unwrap().as_str(), max);
    assert!(ProtocolDiagnostic::new(&oversized).is_err());
}

#[test]
fn diagnostic_limit_is_measured_in_utf8_bytes() {
    let value = "é".repeat(256);
    let oversized = format!("{value}a");

    assert!(ProtocolDiagnostic::new(&value).is_ok());
    assert!(ProtocolDiagnostic::new(&oversized).is_err());
}
