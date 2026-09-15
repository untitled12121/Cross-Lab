use crosslab_core::{
    SessionAuthError, SessionError, SessionState, TransportConnection, TransportSecurityClass,
};
use crosslab_m9_networking::scenarios::auth::{
    AuthAttempt, AuthFixture, BootstrapError, authenticate_direct_pair, bootstrap_direct_pair,
};
use crosslab_policy::NetworkClass;

#[tokio::test]
async fn trusted_peers_activate_over_iroh_exporter() {
    let fixture = AuthFixture::new();
    let pair = authenticate_direct_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("trusted peers should authenticate over Iroh");

    assert_eq!(pair.client_session().state(), SessionState::Active);
    assert_eq!(pair.server_session().state(), SessionState::Active);
    let client_context = pair.client_session().context().unwrap();
    let server_context = pair.server_session().context().unwrap();
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
        pair.client_transport().channel_binding(),
        pair.server_transport().channel_binding()
    );
    assert_eq!(pair.network_class(), NetworkClass::Remote);

    pair.shutdown().await;
}

#[tokio::test]
async fn wrong_connection_binding_fails_before_active() {
    let fixture = AuthFixture::new();
    let rejected = match authenticate_direct_pair(&fixture, AuthAttempt::WrongBinding).await {
        Ok(pair) => {
            pair.shutdown().await;
            panic!("session authentication accepted a proof bound to another connection")
        }
        Err(rejected) => rejected,
    };

    assert_eq!(rejected.client_session().state(), SessionState::Closed);
    assert_eq!(rejected.server_session().state(), SessionState::Closed);
    assert_eq!(
        rejected.error(),
        SessionError::Auth(SessionAuthError::WrongProofTranscript)
    );
}

#[tokio::test]
async fn replay_after_reconnect_fails() {
    let fixture = AuthFixture::new();
    let first = authenticate_direct_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("first authenticated Iroh connection should succeed");
    let old_session_id = first.client_session().context().unwrap().session_id();
    let old_initiator_proof = first.initiator_replay_proof();
    first.shutdown().await;

    let rejected =
        match authenticate_direct_pair(&fixture, AuthAttempt::ReplayInitiator(old_initiator_proof))
            .await
        {
            Ok(pair) => {
                pair.shutdown().await;
                panic!("fresh Iroh connection accepted a proof from the previous connection")
            }
            Err(rejected) => rejected,
        };
    assert_eq!(rejected.client_session().state(), SessionState::Closed);
    assert_eq!(rejected.server_session().state(), SessionState::Closed);
    assert_eq!(
        rejected.error(),
        SessionError::Auth(SessionAuthError::WrongProofTranscript)
    );

    let reconnect = authenticate_direct_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("fresh authentication after reconnect should succeed");
    assert_ne!(
        old_session_id,
        reconnect.client_session().context().unwrap().session_id()
    );
    reconnect.shutdown().await;
}

#[tokio::test]
async fn promotion_is_refused_before_both_sessions_are_active() {
    let fixture = AuthFixture::new();
    let bootstrap = bootstrap_direct_pair(&fixture)
        .await
        .expect("Iroh bootstrap pair");

    assert_eq!(bootstrap.client_session().state(), SessionState::Created);
    assert_eq!(bootstrap.server_session().state(), SessionState::Created);
    assert_eq!(
        bootstrap.try_promote(),
        Err(BootstrapError::SessionNotActive)
    );

    bootstrap.shutdown().await;
}
