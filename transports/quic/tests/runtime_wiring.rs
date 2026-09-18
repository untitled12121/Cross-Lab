use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use crosslab_core::SessionState;
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_mobile_ffi::{
    MobileConnectivityState, MobileLifecycleState, MobileRuntime, MobileSessionState,
    MobileTrustState,
};
use crosslab_policy::{
    NetworkClass, PairingTrustTransition, PolicyState, TransitionId, TrustRecord, TrustTransition,
};
use crosslab_protocol::{FeatureSet, ProtocolRange};
use crosslab_runtime::RuntimeNode;
use crosslab_transport_quic::{
    AuthenticatedQuicSession, QuicClientEndpoint, QuicClientTlsConfig, QuicServerEndpoint,
    QuicServerTlsConfig, QuicSessionAuthConfig, QuicSessionError, QuicSessionTimeouts,
    QuicTransportConfig,
};

const WAIT: Duration = Duration::from_secs(2);
const STATE_CAPACITY: usize = 8;

struct ProvisionedPeers {
    authority: OwnerAuthorityState,
    issuer_key: SigningKey,
    client_key: SigningKey,
    server_key: SigningKey,
    client_credential: DeviceCredential,
    server_credential: DeviceCredential,
    client_trust: TrustRecord,
    server_trust: TrustRecord,
}

impl ProvisionedPeers {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x41; 32]);
        let root_key = SigningKey::from_secret_bytes([0x42; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x43; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let client_key = SigningKey::from_secret_bytes([0x44; 32]);
        let server_key = SigningKey::from_secret_bytes([0x45; 32]);
        let client_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x46; 32]),
            &client_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let server_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x47; 32]),
            &server_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();

        let client_trust = establish_trust(
            &client_credential,
            &authority,
            &issuer_key,
            [0x48; 32],
        );
        let server_trust = establish_trust(
            &server_credential,
            &authority,
            &issuer_key,
            [0x49; 32],
        );

        Self {
            authority,
            issuer_key,
            client_key,
            server_key,
            client_credential,
            server_credential,
            client_trust,
            server_trust,
        }
    }

    fn client_auth<'a>(
        &'a self,
        server_trust: &'a TrustRecord,
    ) -> QuicSessionAuthConfig<'a> {
        QuicSessionAuthConfig::new(
            &self.authority,
            self.client_credential,
            &self.client_key,
            server_trust,
            protocol_ranges(),
            features(),
        )
    }

    fn server_auth(&self) -> QuicSessionAuthConfig<'_> {
        QuicSessionAuthConfig::new(
            &self.authority,
            self.server_credential,
            &self.server_key,
            &self.client_trust,
            protocol_ranges(),
            features(),
        )
    }

    fn revoked_server_trust(&self) -> TrustRecord {
        let mut trust = self.server_trust;
        let transition = TrustTransition::issue_delegated_revocation(
            &trust,
            TransitionId::from_bytes([0x4a; 32]),
            &self.authority,
            AuthorityRole::DeviceSigning,
            &self.issuer_key,
        )
        .unwrap();
        transition
            .apply_delegated(&mut trust, &self.authority)
            .unwrap();
        trust
    }
}

#[tokio::test]
async fn product_quinn_sessions_drive_runtime_and_mobile_status_fail_closed() {
    let peers = ProvisionedPeers::new();
    let (client_endpoint, server_endpoint) = explicit_tls_endpoints();
    let server_addr = server_endpoint.local_addr().unwrap();

    let (first_client, first_server) =
        connect_pair(&peers, &client_endpoint, &server_endpoint, server_addr, &peers.server_trust)
            .await;

    let mut client_runtime = runtime_node(first_client, &peers.server_trust);
    let mut server_runtime = runtime_node(first_server, &peers.client_trust);
    let first_client_status = client_runtime.status(&peers.server_trust);
    let first_server_status = server_runtime.status(&peers.client_trust);

    assert_eq!(first_client_status.session_state(), SessionState::Active);
    assert_eq!(first_server_status.session_state(), SessionState::Active);
    assert_eq!(first_client_status.session_id(), first_server_status.session_id());
    assert_eq!(
        first_client_status.peer_device_id(),
        first_server_status.local_device_id()
    );
    assert_eq!(
        first_server_status.peer_device_id(),
        first_client_status.local_device_id()
    );

    let mobile = MobileRuntime::new();
    mobile.start().unwrap();
    mobile.publish_runtime_status(&first_client_status).unwrap();
    let connected = mobile.snapshot().unwrap();
    assert_eq!(connected.lifecycle, MobileLifecycleState::Running);
    assert_eq!(connected.trust, MobileTrustState::Trusted);
    assert_eq!(connected.connectivity, MobileConnectivityState::Connected);
    assert_eq!(connected.session, MobileSessionState::Active);
    assert!(connected.session_id.is_some());

    let first_session_id = first_client_status.session_id().unwrap();
    client_runtime.network_lost();
    server_runtime.network_lost();

    let disconnected = client_runtime.status(&peers.server_trust);
    mobile.publish_runtime_status(&disconnected).unwrap();
    let mobile_disconnected = mobile.snapshot().unwrap();
    assert_eq!(disconnected.session_state(), SessionState::Closed);
    assert!(disconnected.session_id().is_none());
    assert_eq!(
        mobile_disconnected.connectivity,
        MobileConnectivityState::Disconnected
    );
    assert_eq!(mobile_disconnected.session, MobileSessionState::Closed);
    assert!(mobile_disconnected.session_id.is_none());

    let (second_client, second_server) =
        connect_pair(&peers, &client_endpoint, &server_endpoint, server_addr, &peers.server_trust)
            .await;
    let mut second_client_runtime = runtime_node(second_client, &peers.server_trust);
    let mut second_server_runtime = runtime_node(second_server, &peers.client_trust);
    let second_status = second_client_runtime.status(&peers.server_trust);

    assert_ne!(second_status.session_id(), Some(first_session_id));
    assert_eq!(second_status.session_state(), SessionState::Active);

    let revoked_server_trust = peers.revoked_server_trust();
    second_client_runtime
        .apply_peer_revocation(&revoked_server_trust)
        .unwrap();
    second_server_runtime.network_lost();

    let revoked_status = second_client_runtime.status(&revoked_server_trust);
    mobile.publish_runtime_status(&revoked_status).unwrap();
    let mobile_revoked = mobile.snapshot().unwrap();
    assert_eq!(mobile_revoked.trust, MobileTrustState::Revoked);
    assert_eq!(
        mobile_revoked.connectivity,
        MobileConnectivityState::Disconnected
    );
    assert_eq!(mobile_revoked.session, MobileSessionState::Closed);
    assert!(mobile_revoked.session_id.is_none());

    let client_auth = peers.client_auth(&revoked_server_trust);
    let server_auth = peers.server_auth();
    let (client_result, server_result) = tokio::join!(
        client_endpoint.connect_authenticated(
            server_addr,
            "localhost",
            &client_auth,
            session_timeouts(),
        ),
        server_endpoint.accept_authenticated(&server_auth, session_timeouts()),
    );

    assert!(matches!(
        client_result,
        Err(QuicSessionError::Session(
            crosslab_core::SessionError::PeerNotTrusted
        ))
    ));
    if let Ok(session) = server_result {
        session.transport().shutdown().await;
    }

    let rendered = format!(
        "{:?} {:?} {:?}",
        revoked_status,
        mobile_revoked,
        mobile.poll_event().unwrap()
    )
    .to_ascii_lowercase();
    for forbidden in [
        "channel_binding",
        "credential",
        "private_key",
        "signing_key",
        "tls private",
        "payload",
        "quinn connection",
    ] {
        assert!(!rendered.contains(forbidden), "presentation leaked {forbidden}");
    }

    mobile.stop().unwrap();
}

fn runtime_node(session: AuthenticatedQuicSession, peer_trust: &TrustRecord) -> RuntimeNode<'static> {
    let (session, transport) = session.into_parts();
    let runtime = RuntimeNode::new_owned(
        session,
        Arc::new(transport),
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();

    let status = runtime.status(peer_trust);
    assert_eq!(status.trust_state(), crosslab_policy::TrustState::Trusted);
    runtime
}

async fn connect_pair(
    peers: &ProvisionedPeers,
    client: &QuicClientEndpoint,
    server: &QuicServerEndpoint,
    server_addr: SocketAddr,
    server_trust: &TrustRecord,
) -> (AuthenticatedQuicSession, AuthenticatedQuicSession) {
    let client_auth = peers.client_auth(server_trust);
    let server_auth = peers.server_auth();
    let (client_result, server_result) = tokio::join!(
        client.connect_authenticated(
            server_addr,
            "localhost",
            &client_auth,
            session_timeouts(),
        ),
        server.accept_authenticated(&server_auth, session_timeouts()),
    );
    (
        client_result.expect("client should authenticate"),
        server_result.expect("server should authenticate"),
    )
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

fn explicit_tls_endpoints() -> (QuicClientEndpoint, QuicServerEndpoint) {
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
        QuicClientTlsConfig::new(vec![certificate_der]),
        QuicTransportConfig::default(),
    )
    .unwrap();
    (client, server)
}

fn protocol_ranges() -> Vec<ProtocolRange> {
    vec![ProtocolRange::new(1, 0, 2).unwrap()]
}

fn features() -> FeatureSet {
    FeatureSet::new(&[2], &[]).unwrap()
}

fn session_timeouts() -> QuicSessionTimeouts {
    QuicSessionTimeouts::new(WAIT, WAIT)
}

fn localhost_ephemeral() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}
