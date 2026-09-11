use crosslab_protocol::{
    ProtocolDiagnostic, ProtocolErrorCode, ProtocolErrorCodeError, ProtocolFailure,
};

#[test]
fn protocol_error_codes_have_stable_v1_numeric_values() {
    let cases = [
        (ProtocolErrorCode::MalformedFrame, 1),
        (ProtocolErrorCode::FrameTooLarge, 2),
        (ProtocolErrorCode::UnsupportedMessage, 3),
        (ProtocolErrorCode::IncompatibleProtocol, 4),
        (ProtocolErrorCode::UnsupportedRequiredFeature, 5),
        (ProtocolErrorCode::InvalidIdentifier, 6),
        (ProtocolErrorCode::InvalidSequence, 7),
        (ProtocolErrorCode::ReplayDetected, 8),
        (ProtocolErrorCode::DuplicateRequest, 9),
        (ProtocolErrorCode::InvalidSession, 10),
        (ProtocolErrorCode::AuthenticationFailed, 11),
        (ProtocolErrorCode::TrustDenied, 12),
        (ProtocolErrorCode::AuthorizationDenied, 13),
        (ProtocolErrorCode::CapabilityUnsupported, 14),
        (ProtocolErrorCode::CapabilityVersionIncompatible, 15),
        (ProtocolErrorCode::OperationMissing, 16),
        (ProtocolErrorCode::OperationExpired, 17),
        (ProtocolErrorCode::OperationRevoked, 18),
        (ProtocolErrorCode::OperationMismatch, 19),
        (ProtocolErrorCode::Cancelled, 20),
        (ProtocolErrorCode::ResourceLimit, 21),
        (ProtocolErrorCode::InternalFailure, 22),
    ];

    for (code, value) in cases {
        assert_eq!(code.code(), value);
        assert_eq!(ProtocolErrorCode::from_code(value).unwrap(), code);
    }
    assert_eq!(
        ProtocolErrorCode::from_code(0).unwrap_err(),
        ProtocolErrorCodeError::UnknownValue(0)
    );
}

#[test]
fn protocol_failure_carries_only_typed_code_and_safe_diagnostic() {
    let diagnostic = ProtocolDiagnostic::new("operation expired").unwrap();
    let failure = ProtocolFailure::new(ProtocolErrorCode::OperationExpired, Some(diagnostic));

    assert_eq!(failure.code(), ProtocolErrorCode::OperationExpired);
    assert_eq!(
        failure.diagnostic().map(ProtocolDiagnostic::as_str),
        Some("operation expired")
    );
}
