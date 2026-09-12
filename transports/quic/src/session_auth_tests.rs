use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use crosslab_core::{
    ChannelBinding, ConnectionMetadata, LogicalSession, SessionActivation, SessionAuthError,
    SessionAuthProof, SessionAuthRole as CoreSessionAuthRole, SessionAuthTranscriptV1,
    SessionError, SessionHandshakeSide, SessionState, TransportConnection, TransportSecurityClass,
};
use crosslab_crypto::{SigningKey, blake3_256};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{TransitionId, TrustRecord};
use crosslab_protocol::{
    FeatureSet, FrameLimit, ProtocolRange, SessionAuthBootstrapMessage, SessionAuthHello,
    SessionAuthProofMessage, SessionAuthRole as WireSessionAuthRole, decode_session_auth_bootstrap,
    encode_session_auth_bootstrap, negotiate_features, negotiate_protocol_version,
};
use quinn::{ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig, VarInt};
use rustls::{RootCertStore, pki_types::PrivatePkcs8KeyDer};

use crate::{
    QuicTransportConfig,
    binding::derive_channel_binding,
    connection::QuicTransportConnection,
    record::{read_record, write_record},
};

const BOOTSTRAP_RECORD_MAX: usize = FrameLimit::BootstrapHello.max_payload_len() + 4;
const TEST_CLOSE_CODE: VarInt = VarInt::from_u32(0);

struct AuthFixture {
    owner_id: OwnerId,
    root: OwnerRootRecord,
    delegation: AuthorityDelegation,
    initiator_key: SigningKey,
    responder_key: SigningKey,
    initiator_credential: DeviceCredential,
    responder_credential: DeviceCredential,
    initiator_trust: TrustRecord,
    responder_trust: TrustRecord,
}

impl AuthFixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x40; 32]);
        let root_key = SigningKey::from_secret_bytes([0x41; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x42; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let initiator_key = SigningKey::from_secret_bytes([0x43; 32]);
        let responder_key = SigningKey::from_secret_bytes([0x44; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x45; 32]),
            &initiator_key,
            2,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x46; 32]),
            &responder_key,
            5,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let initiator_trust = TrustRecord::trusted(
            owner_id,
            initiator_credential.device_id(),
            initiator_credential.credential_epoch(),
            TransitionId::from_bytes([0x47; 32]),
        );
        let responder_trust = TrustRecord::trusted(
            owner_id,
            responder_credential.device_id(),
            responder_credential.credential_epoch(),
            TransitionId::from_bytes([0x48; 32]),
        );

        Self {
            owner_id,
            root,
            delegation,
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
            initiator_trust,
            responder_trust,
        }
    }

    fn initiator_ranges() -> Vec<ProtocolRange> {
        vec![ProtocolRange::new(1, 0, 3).unwrap()]
    }

    fn responder_ranges() -> Vec<ProtocolRange> {
        vec![ProtocolRange::new(1, 1, 2).unwrap()]
    }

    fn initiator_features() -> FeatureSet {
        FeatureSet::new(&[1, 2, 3], &[2]).unwrap()
    }

    fn responder_features() -> FeatureSet {
        FeatureSet::new(&[2, 3, 4], &[3]).unwrap()
    }
}

#[derive(Clone, Copy)]
struct ReplayProof {
    proof: SessionAuthProof,
    message: SessionAuthProofMessage,
}

#[derive(Clone, Copy)]
enum AuthAttempt {
    Normal,
    WrongBinding,
    ReplayInitiator(ReplayProof),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootstrapTestError {
    SessionNotActive,
}

struct RawLoopbackPair {
    client_endpoint: Endpoint,
    server_endpoint: Endpoint,
    client: Connection,
    server: Connection,
}

struct BootstrapLoopbackSessionPair {
    client_endpoint: Endpoint,
    server_endpoint: Endpoint,
    client_connection: Connection,
    server_connection: Connection,
    client_send: SendStream,
    client_recv: RecvStream,
    server_send: SendStream,
    server_recv: RecvStream,
    client_binding: ChannelBinding,
    server_binding: ChannelBinding,
    client_session: LogicalSession,
    server_session: LogicalSession,
}

impl BootstrapLoopbackSessionPair {
    fn try_promote(&self) -> Result<(), BootstrapTestError> {
        require_active(&self.client_session, &self.server_session)
    }

    async fn shutdown(self) {
        close_raw_connections(&self.client_connection, &self.server_connection).await;
    }
}

struct AuthenticatedLoopbackSessionPair {
    _client_endpoint: Endpoint,
    _server_endpoint: Endpoint,
    client_transport: QuicTransportConnection,
    server_transport: QuicTransportConnection,
    client_session: LogicalSession,
    server_session: LogicalSession,
    initiator_proof_message: ReplayProof,
}

impl AuthenticatedLoopbackSessionPair {
    async fn shutdown(self) {
        self.client_transport.shutdown().await;
        self.server_transport.shutdown().await;
    }
}

#[derive(Debug)]
struct RejectedAuthentication {
    client_session: LogicalSession,
    server_session: LogicalSession,
    error: SessionError,
}

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

async fn authenticate_loopback_session_pair(
    fixture: &AuthFixture,
    attempt: AuthAttempt,
) -> Result<AuthenticatedLoopbackSessionPair, Box<RejectedAuthentication>> {
    let mut bootstrap = bootstrap_loopback_session_pair(fixture).await;

    let initiator_hello = expect_hello(receive_bootstrap(&mut bootstrap.server_recv).await);
    let responder_hello = SessionAuthHello::new(
        fixture.responder_credential,
        AuthFixture::responder_ranges(),
        AuthFixture::responder_features(),
        nonce_for_binding(&bootstrap.server_binding, b"responder"),
    );
    send_bootstrap(
        &mut bootstrap.server_send,
        &SessionAuthBootstrapMessage::Hello(responder_hello.clone()),
    )
    .await;
    let responder_hello = expect_hello(receive_bootstrap(&mut bootstrap.client_recv).await);

    let actual_transcript = transcript_for(
        fixture,
        &initiator_hello,
        &responder_hello,
        &bootstrap.client_binding,
    );
    let proof_transcript = match attempt {
        AuthAttempt::WrongBinding => {
            let mut bytes = bootstrap.client_binding.bytes().to_vec();
            bytes[0] ^= 0xFF;
            let wrong_binding = ChannelBinding::new(bootstrap.client_binding.profile_id(), bytes);
            transcript_for(fixture, &initiator_hello, &responder_hello, &wrong_binding)
        }
        AuthAttempt::Normal | AuthAttempt::ReplayInitiator(_) => actual_transcript.clone(),
    };

    let generated_initiator_proof = proof_transcript
        .create_proof(CoreSessionAuthRole::Initiator, &fixture.initiator_key)
        .unwrap();
    let responder_proof = proof_transcript
        .create_proof(CoreSessionAuthRole::Responder, &fixture.responder_key)
        .unwrap();
    let generated_replay = ReplayProof {
        proof: generated_initiator_proof,
        message: proof_message(generated_initiator_proof),
    };
    let initiator_replay = match attempt {
        AuthAttempt::ReplayInitiator(replay) => replay,
        AuthAttempt::Normal | AuthAttempt::WrongBinding => generated_replay,
    };
    let responder_message = proof_message(responder_proof);

    send_bootstrap(
        &mut bootstrap.client_send,
        &SessionAuthBootstrapMessage::Proof(initiator_replay.message),
    )
    .await;
    send_bootstrap(
        &mut bootstrap.server_send,
        &SessionAuthBootstrapMessage::Proof(responder_message),
    )
    .await;
    assert_eq!(
        expect_proof(receive_bootstrap(&mut bootstrap.server_recv).await),
        initiator_replay.message
    );
    assert_eq!(
        expect_proof(receive_bootstrap(&mut bootstrap.client_recv).await),
        responder_message
    );

    let initiator_credential = initiator_hello.device_credential();
    let responder_credential = responder_hello.device_credential();
    let initiator_side = SessionHandshakeSide::new(
        &initiator_credential,
        &fixture.delegation,
        initiator_hello.protocol_ranges(),
        initiator_hello.features(),
    );
    let responder_side = SessionHandshakeSide::new(
        &responder_credential,
        &fixture.delegation,
        responder_hello.protocol_ranges(),
        responder_hello.features(),
    );

    let client_result = bootstrap
        .client_session
        .authenticate(SessionActivation::new(
            &fixture.root,
            initiator_side,
            responder_side,
            CoreSessionAuthRole::Initiator,
            &fixture.responder_trust,
            initiator_hello.nonce(),
            responder_hello.nonce(),
            &bootstrap.client_binding,
            TransportSecurityClass::AuthenticatedConfidentialChannel,
            &initiator_replay.proof,
            &responder_proof,
        ));
    let server_result = bootstrap
        .server_session
        .authenticate(SessionActivation::new(
            &fixture.root,
            initiator_side,
            responder_side,
            CoreSessionAuthRole::Responder,
            &fixture.initiator_trust,
            initiator_hello.nonce(),
            responder_hello.nonce(),
            &bootstrap.server_binding,
            TransportSecurityClass::AuthenticatedConfidentialChannel,
            &initiator_replay.proof,
            &responder_proof,
        ));

    match (client_result, server_result) {
        (Ok(()), Ok(())) => promote_authenticated(bootstrap, initiator_replay)
            .map_err(|_| unreachable!("active authenticated sessions must be promotable")),
        (Err(client_error), Err(server_error)) => {
            assert_eq!(client_error, server_error);
            bootstrap.close_connections().await;
            let BootstrapLoopbackSessionPair {
                client_session,
                server_session,
                ..
            } = bootstrap;
            Err(Box::new(RejectedAuthentication {
                client_session,
                server_session,
                error: client_error,
            }))
        }
        _ => panic!("peers disagreed on session authentication outcome"),
    }
}

async fn bootstrap_loopback_session_pair(fixture: &AuthFixture) -> BootstrapLoopbackSessionPair {
    let raw = loopback_connection_pair().await;
    let client_binding = derive_channel_binding(&raw.client).unwrap();
    let server_binding = derive_channel_binding(&raw.server).unwrap();
    assert_eq!(client_binding, server_binding);

    let initiator_hello = SessionAuthHello::new(
        fixture.initiator_credential,
        AuthFixture::initiator_ranges(),
        AuthFixture::initiator_features(),
        nonce_for_binding(&client_binding, b"initiator"),
    );
    let (mut client_send, client_recv) = raw.client.open_bi().await.unwrap();
    send_bootstrap(
        &mut client_send,
        &SessionAuthBootstrapMessage::Hello(initiator_hello),
    )
    .await;
    let (server_send, server_recv) = raw.server.accept_bi().await.unwrap();

    BootstrapLoopbackSessionPair {
        client_endpoint: raw.client_endpoint,
        server_endpoint: raw.server_endpoint,
        client_connection: raw.client,
        server_connection: raw.server,
        client_send,
        client_recv,
        server_send,
        server_recv,
        client_binding,
        server_binding,
        client_session: LogicalSession::new(),
        server_session: LogicalSession::new(),
    }
}

impl BootstrapLoopbackSessionPair {
    async fn close_connections(&self) {
        close_raw_connections(&self.client_connection, &self.server_connection).await;
    }
}

fn promote_authenticated(
    bootstrap: BootstrapLoopbackSessionPair,
    initiator_proof_message: ReplayProof,
) -> Result<AuthenticatedLoopbackSessionPair, BootstrapTestError> {
    require_active(&bootstrap.client_session, &bootstrap.server_session)?;

    let client_local = bootstrap.client_endpoint.local_addr().unwrap();
    let server_local = bootstrap.server_endpoint.local_addr().unwrap();
    let client_transport = QuicTransportConnection::new(
        bootstrap.client_connection.clone(),
        bootstrap.client_send,
        bootstrap.client_recv,
        bootstrap.client_binding,
        ConnectionMetadata::new(
            Some(client_local.to_string()),
            Some(server_local.to_string()),
            Some(false),
        ),
        QuicTransportConfig::default(),
    );
    let server_transport = QuicTransportConnection::new(
        bootstrap.server_connection.clone(),
        bootstrap.server_send,
        bootstrap.server_recv,
        bootstrap.server_binding,
        ConnectionMetadata::new(
            Some(server_local.to_string()),
            Some(client_local.to_string()),
            Some(false),
        ),
        QuicTransportConfig::default(),
    );

    Ok(AuthenticatedLoopbackSessionPair {
        _client_endpoint: bootstrap.client_endpoint,
        _server_endpoint: bootstrap.server_endpoint,
        client_transport,
        server_transport,
        client_session: bootstrap.client_session,
        server_session: bootstrap.server_session,
        initiator_proof_message,
    })
}

fn require_active(
    client: &LogicalSession,
    server: &LogicalSession,
) -> Result<(), BootstrapTestError> {
    if client.state() == SessionState::Active && server.state() == SessionState::Active {
        Ok(())
    } else {
        Err(BootstrapTestError::SessionNotActive)
    }
}

fn transcript_for(
    fixture: &AuthFixture,
    initiator: &SessionAuthHello,
    responder: &SessionAuthHello,
    binding: &ChannelBinding,
) -> SessionAuthTranscriptV1 {
    let initiator_credential = initiator.device_credential();
    let responder_credential = responder.device_credential();
    let protocol =
        negotiate_protocol_version(initiator.protocol_ranges(), responder.protocol_ranges())
            .unwrap();
    let features = negotiate_features(initiator.features(), responder.features()).unwrap();

    SessionAuthTranscriptV1::new(
        fixture.owner_id,
        &initiator_credential,
        initiator.nonce(),
        &responder_credential,
        responder.nonce(),
        protocol,
        &features,
        binding.profile_id().as_bytes(),
        binding.bytes(),
    )
    .unwrap()
}

fn proof_message(proof: SessionAuthProof) -> SessionAuthProofMessage {
    let role = match proof.role() {
        CoreSessionAuthRole::Initiator => WireSessionAuthRole::Initiator,
        CoreSessionAuthRole::Responder => WireSessionAuthRole::Responder,
    };
    SessionAuthProofMessage::new(role, proof.transcript_digest(), proof.signature())
}

async fn send_bootstrap(send: &mut SendStream, message: &SessionAuthBootstrapMessage) {
    let frame = encode_session_auth_bootstrap(message).unwrap();
    write_record(send, &frame, BOOTSTRAP_RECORD_MAX)
        .await
        .unwrap();
}

async fn receive_bootstrap(recv: &mut RecvStream) -> SessionAuthBootstrapMessage {
    let frame = read_record(recv, BOOTSTRAP_RECORD_MAX, false)
        .await
        .unwrap();
    decode_session_auth_bootstrap(&frame).unwrap()
}

fn expect_hello(message: SessionAuthBootstrapMessage) -> SessionAuthHello {
    match message {
        SessionAuthBootstrapMessage::Hello(hello) => hello,
        SessionAuthBootstrapMessage::Proof(_) => panic!("expected session-auth hello"),
    }
}

fn expect_proof(message: SessionAuthBootstrapMessage) -> SessionAuthProofMessage {
    match message {
        SessionAuthBootstrapMessage::Proof(proof) => proof,
        SessionAuthBootstrapMessage::Hello(_) => panic!("expected session-auth proof"),
    }
}

fn nonce_for_binding(binding: &ChannelBinding, role: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(binding.bytes().len() + role.len());
    input.extend_from_slice(binding.bytes());
    input.extend_from_slice(role);
    blake3_256(&input)
}

async fn close_raw_connections(client: &Connection, server: &Connection) {
    client.close(TEST_CLOSE_CODE, b"session auth test complete");
    server.close(TEST_CLOSE_CODE, b"session auth test complete");
    let _ = tokio::join!(client.closed(), server.closed());
}

async fn loopback_connection_pair() -> RawLoopbackPair {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let certificate = certified.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let server_config =
        ServerConfig::with_single_cert(vec![certificate.clone()], key.into()).unwrap();
    let server_endpoint = Endpoint::server(
        server_config,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
    )
    .unwrap();
    let server_addr = server_endpoint.local_addr().unwrap();

    let mut roots = RootCertStore::empty();
    roots.add(certificate).unwrap();
    let mut client_endpoint =
        Endpoint::client(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).unwrap();
    client_endpoint
        .set_default_client_config(ClientConfig::with_root_certificates(Arc::new(roots)).unwrap());

    let client_connecting = client_endpoint.connect(server_addr, "localhost").unwrap();
    let server_incoming = server_endpoint.accept().await.unwrap();
    let (client, server) = tokio::join!(client_connecting, server_incoming);

    RawLoopbackPair {
        client_endpoint,
        server_endpoint,
        client: client.unwrap(),
        server: server.unwrap(),
    }
}
