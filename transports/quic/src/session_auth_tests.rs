use crosslab_core::{SessionAuthError, SessionError, SessionState, TransportSecurityClass};

#[tokio::test]
async fn session_auth_quinn_trusted_peers_activate() {
    let fixture = AuthFixture::new();
    let pair = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("trusted peers should authenticate");

    assert_eq!(pair.client_session.state(), SessionState::Active);
    assert_eq!(pair.server_session.state(), SessionState::Active);
    let client_context = pair.client_session.context().unwrap();
    let server_context = pair.server_session.context().unwrap();
    assert_eq!(client_context.session_id(), server_context.session_id());
    assert_eq!(
        client_context.transport_security_class(),
        TransportSecurityClass::AuthenticatedConfidentialChannel
    );
    assert_eq!(
        server_context.transport_security_class(),
        TransportSecurityClass::AuthenticatedConfidentialChannel
    );
    assert_eq!(
        pair.client_transport.channel_binding(),
        pair.server_transport.channel_binding()
    );

    pair.shutdown().await;
}

#[tokio::test]
async fn session_auth_wrong_connection_binding_fails_before_active() {
    let fixture = AuthFixture::new();
    let rejected =
        match authenticate_loopback_session_pair(&fixture, AuthAttempt::WrongBinding).await {
            Ok(pair) => {
                pair.shutdown().await;
                panic!("session authentication accepted a proof bound to another channel")
            }
            Err(rejected) => rejected,
        };

    assert_eq!(rejected.client_session.state(), SessionState::Closed);
    assert_eq!(rejected.server_session.state(), SessionState::Closed);
    assert_eq!(
        rejected.error,
        SessionError::Auth(SessionAuthError::WrongProofTranscript)
    );
}

#[tokio::test]
async fn session_auth_replayed_proof_on_reconnect_fails_before_active() {
    let fixture = AuthFixture::new();
    let first = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("first authenticated connection should succeed");
    let old_session_id = first.client_session.context().unwrap().session_id();
    let old_initiator_proof = first.initiator_proof_message;
    first.shutdown().await;

    let rejected = match authenticate_loopback_session_pair(
        &fixture,
        AuthAttempt::ReplayInitiator(old_initiator_proof),
    )
    .await
    {
        Ok(pair) => {
            pair.shutdown().await;
            panic!("fresh connection accepted a proof from the previous connection")
        }
        Err(rejected) => rejected,
    };
    assert_eq!(rejected.client_session.state(), SessionState::Closed);
    assert_eq!(rejected.server_session.state(), SessionState::Closed);
    assert_eq!(
        rejected.error,
        SessionError::Auth(SessionAuthError::WrongProofTranscript)
    );

    let reconnect = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("fresh authentication after reconnect should succeed");
    assert_ne!(
        old_session_id,
        reconnect.client_session.context().unwrap().session_id()
    );
    reconnect.shutdown().await;
}

#[tokio::test]
async fn session_auth_control_bridge_requires_active_sessions() {
    let fixture = AuthFixture::new();
    let bootstrap = bootstrap_loopback_session_pair(&fixture).await;

    assert_eq!(bootstrap.client_session.state(), SessionState::Created);
    assert_eq!(bootstrap.server_session.state(), SessionState::Created);
    assert_eq!(
        bootstrap.try_promote(),
        Err(BootstrapTestError::SessionNotActive)
    );

    bootstrap.shutdown().await;
}
