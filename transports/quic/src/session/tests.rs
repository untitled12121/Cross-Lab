use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use crosslab_core::{SessionError, SessionState, TransportConnection};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    PairingTrustTransition, TransitionId, TrustRecord, TrustTransition,
};
use crosslab_protocol::{
    FeatureSet, ProtocolRange, SessionAuthBootstrapMessage, SessionAuthHello,
};
use quinn::{ClientConfig, Connection, Endpoint, RecvStream, SendStream};
use rustls::{RootCertStore, pki_types::CertificateDer};

use super::{BOOTSTRAP_RECORD_MAX, receive_bootstrap, send_bootstrap};
use crate::{
    AuthenticatedQuicSession, QuicClientEndpoint, QuicClientTlsConfig, QuicServerEndpoint,
    QuicServerTlsConfig, QuicSessionAuthConfig, QuicSessionError, QuicSessionTimeouts,
    QuicTransportConfig,
};

const WAIT: Duration = Duration::from_secs(2);

struct AuthFixture {
    authority: OwnerAuthorityState,
    issuer_key: SigningKey,
    initiator_key: SigningKey,
    responder_key: SigningKey,
    initiator_credential: DeviceCredential,
    responder_credential: DeviceCredential,
    initiator_trust: TrustRecord,
    responder_trust: TrustRecord,
}

impl AuthFixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x20; 32]);
        let root_key = SigningKey::from_secret_bytes([0x21; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x22; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let initiator_key = SigningKey::from_secret_bytes([0x23; 32]);
        let responder_key = SigningKey::from_secret_bytes([0x24; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x25; 32]),
            &initiator_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x26; 32]),
            &responder_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let initiator_trust =
            establish_trust(&initiator_credential, &authority, &issuer_key, [0x27; 32]);
        let responder_trust =
            establish_trust(&responder_credential, &authority, &issuer_key, [0x28; 32]);

        Self {
            authority,
            issuer_key,
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
            initiator_trust,
            responder_trust,
        }
    }

    fn initiator_auth(&self) -> QuicSessionAuthConfig<'_> {
        self.initiator_auth_with_peer_trust(&self.responder_trust)
    }

    fn initiator_auth_with_peer_trust<'a>(
        &'a self,
        peer_trust: &'a TrustRecord,
    ) -> QuicSessionAuthConfig<'a> {
        QuicSessionAuthConfig::new(
            &self.authority,
            self.initiator_credential,
            &self.initiator_key,
            peer_trust,
            initiator_ranges(),
            initiator_features(),
        )
    }

    fn responder_auth(&self) -> QuicSessionAuthConfig<'_> {
        QuicSessionAuthConfig::new(
            &self.authority,
            self.responder_credential,
            &self.responder_key,
            &self.initiator_trust,
            responder_ranges(),
            responder_features(),
        )
    }

    fn revoked_responder_trust(&self) -> TrustRecord {
        let mut trust = self.responder_trust;
        let revocation = TrustTransition::issue_delegated_revocation(
            &trust,
            TransitionId::from_bytes([0x29; 32]),
            &self.authority,
            AuthorityRole::DeviceSigning,
            &self.issuer_key,
        )
        .unwrap();
        revocation.apply_delegated(&mut trust, &self.authority).unwrap();
        trust
    }
}

#[tokio::test]
async fn public_endpoints_return_only_authenticated_sessions() {
    let fixture = AuthFixture::new();
    let (client, server, _) = explicit_tls_endpoints();
    let server_addr = server.local_addr().unwrap();
    let timeouts = session_timeouts();
    let initiator_auth = fixture.initiator_auth();
    let responder_auth = fixture.responder_auth();

    let (client_result, server_result) = tokio::join!(
        client.connect_authenticated(server_addr, "localhost", &initiator_auth, timeouts),
        server.accept_authenticated(&responder_auth, timeouts),
    );
    let client_session = client_result.expect("trusted client should authenticate");
    let server_session = server_result.expect("trusted server should authenticate");

    assert_eq!(client_session.session().state(), SessionState::Active);
    assert_eq!(server_session.session().state(), SessionState::Active);
    assert_eq!(
        client_session.session().context().unwrap().session_id(),
        server_session.session().context().unwrap().session_id()
    );
    assert_eq!(
        client_session.transport().channel_binding(),
        server_session.transport().channel_binding()
    );

    shutdown_authenticated(client_session, server_session).await;
}

#[tokio::test]
async fn unknown_peer_is_rejected_before_transport_is_returned() {
    let fixture = AuthFixture::new();
    let (client, server, _) = explicit_tls_endpoints();
    let server_addr = server.local_addr().unwrap();
    let unknown_peer_trust = fixture.initiator_trust;
    let initiator_auth = fixture.initiator_auth_with_peer_trust(&unknown_peer_trust);
    let responder_auth = fixture.responder_auth();

    let (client_result, server_result) = tokio::join!(
        client.connect_authenticated(
            server_addr,
            "localhost",
            &initiator_auth,
            session_timeouts(),
        ),
        server.accept_authenticated(&responder_auth, session_timeouts()),
    );

    assert_rejected(
        client_result,
        QuicSessionError::Session(SessionError::PeerTrustMismatch),
    )
    .await;
    shutdown_if_authenticated(server_result).await;
}

#[tokio::test]
async fn revoked_peer_is_rejected_before_transport_is_returned() {
    let fixture = AuthFixture::new();
    let (client, server, _) = explicit_tls_endpoints();
    let server_addr = server.local_addr().unwrap();
    let revoked_peer_trust = fixture.revoked_responder_trust();
    let initiator_auth = fixture.initiator_auth_with_peer_trust(&revoked_peer_trust);
    let responder_auth = fixture.responder_auth();

    let (client_result, server_result) = tokio::join!(
        client.connect_authenticated(
            server_addr,
            "localhost",
            &initiator_auth,
            session_timeouts(),
        ),
        server.accept_authenticated(&responder_auth, session_timeouts()),
    );

    assert_rejected(
        client_result,
        QuicSessionError::Session(SessionError::PeerNotTrusted),
    )
    .await;
    shutdown_if_authenticated(server_result).await;
}

#[tokio::test]
async fn malformed_bootstrap_frame_is_rejected_and_connection_is_closed() {
    let fixture = AuthFixture::new();
    let (_, server, certificate_der) = explicit_tls_endpoints();
    let server_addr = server.local_addr().unwrap();
    let responder_auth = fixture.responder_auth();

    let server_future = server.accept_authenticated(&responder_auth, session_timeouts());
    let client_future = async {
        let (endpoint, connection) = raw_client_connect(&certificate_der, server_addr).await;
        let (mut send, _recv) = connection.open_bi().await.unwrap();
        send.write_all(&1_u32.to_be_bytes()).await.unwrap();
        send.write_all(&[0xff]).await.unwrap();
        (endpoint, connection)
    };
    let (server_result, (_endpoint, connection)) = tokio::join!(server_future, client_future);

    assert_rejected(server_result, QuicSessionError::Bootstrap).await;
    assert_connection_closes(&connection).await;
}

#[tokio::test]
async fn oversized_bootstrap_frame_is_rejected_and_connection_is_closed() {
    let fixture = AuthFixture::new();
    let (_, server, certificate_der) = explicit_tls_endpoints();
    let server_addr = server.local_addr().unwrap();
    let responder_auth = fixture.responder_auth();

    let server_future = server.accept_authenticated(&responder_auth, session_timeouts());
    let client_future = async {
        let (endpoint, connection) = raw_client_connect(&certificate_der, server_addr).await;
        let (mut send, _recv) = connection.open_bi().await.unwrap();
        let declared = u32::try_from(BOOTSTRAP_RECORD_MAX + 1).unwrap();
        send.write_all(&declared.to_be_bytes()).await.unwrap();
        (endpoint, connection)
    };
    let (server_result, (_endpoint, connection)) = tokio::join!(server_future, client_future);

    assert_rejected(server_result, QuicSessionError::Bootstrap).await;
    assert_connection_closes(&connection).await;
}

#[tokio::test]
async fn bootstrap_timeout_closes_the_connection() {
    let fixture = AuthFixture::new();
    let (_, server, certificate_der) = explicit_tls_endpoints();
    let server_addr = server.local_addr().unwrap();
    let responder_auth = fixture.responder_auth();
    let short_timeout = QuicSessionTimeouts::new(WAIT, Duration::from_millis(100));

    let server_future = server.accept_authenticated(&responder_auth, short_timeout);
    let client_future = raw_client_connect(&certificate_der, server_addr);
    let (server_result, (_endpoint, connection)) = tokio::join!(server_future, client_future);

    assert_rejected(server_result, QuicSessionError::Timeout).await;
    assert_connection_closes(&connection).await;
}

#[tokio::test]
async fn cancelling_bootstrap_closes_the_connection() {
    let fixture = AuthFixture::new();
    let (_, server, certificate_der) = explicit_tls_endpoints();
    let server_addr = server.local_addr().unwrap();
    let responder_auth = fixture.responder_auth();
    let long_timeout = QuicSessionTimeouts::new(WAIT, Duration::from_secs(30));
    let mut accept = Box::pin(server.accept_authenticated(&responder_auth, long_timeout));

    let client = async {
        let (endpoint, connection) = raw_client_connect(&certificate_der, server_addr).await;
        let (mut send, mut recv) = connection.open_bi().await.unwrap();
        let hello = SessionAuthHello::new(
            fixture.initiator_credential,
            initiator_ranges(),
            initiator_features(),
            [0x2a; 32],
        );
        send_bootstrap(
            &mut send,
            &SessionAuthBootstrapMessage::Hello(hello),
        )
        .await
        .unwrap();
        match receive_bootstrap(&mut recv).await.unwrap() {
            SessionAuthBootstrapMessage::Hello(_) => {}
            SessionAuthBootstrapMessage::Proof(_) => panic!("responder sent proof before hello"),
        }
        (endpoint, connection, send, recv)
    };

    let (endpoint, connection, send, recv) = tokio::select! {
        client = client => client,
        _ = &mut accept => panic!("server bootstrap completed before cancellation"),
    };
    drop(accept);
    drop((send, recv));
    assert_connection_closes(&connection).await;
    drop(endpoint);
}

#[tokio::test]
async fn fresh_reconnect_creates_new_binding_and_session_id() {
    let fixture = AuthFixture::new();
    let (client, server, _) = explicit_tls_endpoints();
    let server_addr = server.local_addr().unwrap();

    let initiator_auth = fixture.initiator_auth();
    let responder_auth = fixture.responder_auth();
    let (first_client, first_server) = tokio::join!(
        client.connect_authenticated(
            server_addr,
            "localhost",
            &initiator_auth,
            session_timeouts(),
        ),
        server.accept_authenticated(&responder_auth, session_timeouts()),
    );
    let first_client = first_client.unwrap();
    let first_server = first_server.unwrap();
    let first_session_id = first_client.session().context().unwrap().session_id();
    let first_binding = first_client.transport().channel_binding().bytes().to_vec();
    shutdown_authenticated(first_client, first_server).await;

    let initiator_auth = fixture.initiator_auth();
    let responder_auth = fixture.responder_auth();
    let (second_client, second_server) = tokio::join!(
        client.connect_authenticated(
            server_addr,
            "localhost",
            &initiator_auth,
            session_timeouts(),
        ),
        server.accept_authenticated(&responder_auth, session_timeouts()),
    );
    let second_client = second_client.unwrap();
    let second_server = second_server.unwrap();

    assert_ne!(
        first_session_id,
        second_client.session().context().unwrap().session_id()
    );
    assert_ne!(
        first_binding,
        second_client.transport().channel_binding().bytes()
    );
    shutdown_authenticated(second_client, second_server).await;
}

fn establish_trust(
    credential: &DeviceCredential,
    authority: &OwnerAuthorityState,
    issuer_key: &SigningKey,
    transition_id: [u8; 32],
) -> TrustRecord {
    PairingTrustTransition::issue(
        credential,
        TransitionId::from_bytes(transition_id),
        [transition_id[0].wrapping_add(1); 32],
        authority,
        issuer_key,
    )
    .unwrap()
    .establish(credential, authority)
    .unwrap()
}

fn initiator_ranges() -> Vec<ProtocolRange> {
    vec![ProtocolRange::new(1, 0, 2).unwrap()]
}

fn responder_ranges() -> Vec<ProtocolRange> {
    vec![ProtocolRange::new(1, 1, 3).unwrap()]
}

fn initiator_features() -> FeatureSet {
    FeatureSet::new(&[2, 3], &[2]).unwrap()
}

fn responder_features() -> FeatureSet {
    FeatureSet::new(&[2, 4], &[]).unwrap()
}

fn explicit_tls_endpoints() -> (QuicClientEndpoint, QuicServerEndpoint, Vec<u8>) {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let certificate_der = certified.cert.der().as_ref().to_vec();
    let private_key_pkcs8_der = certified.signing_key.serialize_der();
    let server = QuicServerEndpoint::bind(
        localhost_ephemeral(),
        QuicServerTlsConfig::new(vec![certificate_der.clone()], private_key_pkcs8_der),
        QuicTransportConfig::default(),
    )
    .unwrap();
    let client = QuicClientEndpoint::bind(
        localhost_ephemeral(),
        QuicClientTlsConfig::new(vec![certificate_der.clone()]),
        QuicTransportConfig::default(),
    )
    .unwrap();
    (client, server, certificate_der)
}

async fn raw_client_connect(
    certificate_der: &[u8],
    server_addr: SocketAddr,
) -> (Endpoint, Connection) {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(certificate_der.to_vec()))
        .unwrap();
    let mut endpoint = Endpoint::client(localhost_ephemeral()).unwrap();
    endpoint.set_default_client_config(
        ClientConfig::with_root_certificates(Arc::new(roots)).unwrap(),
    );
    let connection = endpoint
        .connect(server_addr, "localhost")
        .unwrap()
        .await
        .unwrap();
    (endpoint, connection)
}

async fn assert_rejected(
    result: Result<AuthenticatedQuicSession, QuicSessionError>,
    expected: QuicSessionError,
) {
    match result {
        Err(error) => assert_eq!(error, expected),
        Ok(session) => {
            session.transport().shutdown().await;
            panic!("session authentication unexpectedly succeeded");
        }
    }
}

async fn shutdown_if_authenticated(result: Result<AuthenticatedQuicSession, QuicSessionError>) {
    if let Ok(session) = result {
        session.transport().shutdown().await;
    }
}

async fn shutdown_authenticated(
    client: AuthenticatedQuicSession,
    server: AuthenticatedQuicSession,
) {
    client.transport().shutdown().await;
    server.transport().shutdown().await;
}

async fn assert_connection_closes(connection: &Connection) {
    tokio::time::timeout(WAIT, connection.closed())
        .await
        .expect("rejected session should close its QUIC connection");
}

fn session_timeouts() -> QuicSessionTimeouts {
    QuicSessionTimeouts::new(WAIT, WAIT)
}

fn localhost_ephemeral() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}
