use crosslab_protocol::{ProtocolDiagnostic, SessionClose, SessionCloseReason};

#[test]
fn session_close_reason_matches_the_v1_registry() {
    let cases = [
        (SessionCloseReason::Normal, 1),
        (SessionCloseReason::LocalRequest, 2),
        (SessionCloseReason::ProtocolError, 3),
        (SessionCloseReason::AuthenticationLost, 4),
        (SessionCloseReason::TrustRevoked, 5),
        (SessionCloseReason::Shutdown, 6),
    ];

    for (reason, code) in cases {
        assert_eq!(reason.code(), code);
        assert_eq!(SessionCloseReason::from_code(code).unwrap(), reason);
    }

    assert!(SessionCloseReason::from_code(0).is_err());
    assert!(SessionCloseReason::from_code(7).is_err());
}

#[test]
fn session_close_carries_only_a_typed_reason_and_safe_diagnostic() {
    let diagnostic = ProtocolDiagnostic::new("owner requested shutdown").unwrap();
    let close = SessionClose::new(SessionCloseReason::Shutdown, Some(diagnostic.clone()));

    assert_eq!(close.reason(), SessionCloseReason::Shutdown);
    assert_eq!(close.diagnostic(), Some(&diagnostic));
}
