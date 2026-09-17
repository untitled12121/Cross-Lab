use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use crosslab_core::{SessionState, TransportConnection};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId, TrustRecord};
use crosslab_protocol::{FeatureSet, ProtocolRange};

use crate::{
    QuicClientEndpoint, QuicClientTlsConfig, QuicServerEndpoint, QuicServerTlsConfig,
    QuicSessionAuthConfig, QuicSessionTimeouts, QuicTransportConfig,
};

struct AuthFixture {
    authority: OwnerAuthorityState,
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
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
            initiator_trust,
            responder_trust,
        }
    }

    fn initiator_auth(&self) -> QuicSessionAuthConfig<'_> {
        QuicSessionAuthConfig::new(
            &self.authority,
            self.initiator_credential,
            &self.initiator_key,
            &self.responder_trust,
            vec![ProtocolRange::new(1, 0, 2).unwrap()],
            FeatureSet::new(&[2, 3], &[2]).unwrap(),
        )
    }

    fn responder_auth(&self) -> QuicSessionAuthConfig<'_> {
        QuicSessionAuthConfig::new(
            &self.authority,
            self.responder_credential,
            &self.responder_key,
            &self.initiator_trust,
            vec![ProtocolRange::new(1, 1, 3).unwrap()],
            FeatureSet::new(&[2, 4], &[]).unwrap(),
        )
    }
}

#[tokio::test]
async fn public_endpoints_return_only_authenticated_sessions() {
    let fixture = AuthFixture::new();
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
    let server_addr = server.local_addr().unwrap();
    let timeouts = QuicSessionTimeouts::new(Duration::from_secs(2), Duration::from_secs(2));
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

    client_session.transport().shutdown().await;
    server_session.transport().shutdown().await;
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

fn localhost_ephemeral() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}
