use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use crosslab_crypto::{Signature, SigningKey, SigningProvider, SigningProviderError, VerifyingKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId, TrustRecord};
use crosslab_protocol::{FeatureSet, ProtocolRange};
use crosslab_transport_quic::{
    QuicSessionAuthConfig, QuicSessionTimeouts, QuicTransportConfig, TrustedSessionQuicClient,
    TrustedSessionQuicServer,
};

const WAIT: Duration = Duration::from_secs(2);

struct ProviderSigner<'a>(&'a SigningKey);

impl SigningProvider for ProviderSigner<'_> {
    fn verifying_key(&self) -> VerifyingKey {
        self.0.verifying_key()
    }

    fn sign_message(&self, message: &[u8]) -> Result<Signature, SigningProviderError> {
        Ok(self.0.sign_message(message))
    }
}

struct Fixture {
    authority: OwnerAuthorityState,
    issuer: SigningKey,
    client_key: SigningKey,
    server_key: SigningKey,
    client_credential: DeviceCredential,
    server_credential: DeviceCredential,
    client_trust: TrustRecord,
    server_trust: TrustRecord,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x61; 32]);
        let root_key = SigningKey::from_secret_bytes([0x62; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer = SigningKey::from_secret_bytes([0x63; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let client_key = SigningKey::from_secret_bytes([0x64; 32]);
        let server_key = SigningKey::from_secret_bytes([0x65; 32]);
        let client_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x66; 32]),
            &client_key,
            0,
            &authority,
            &issuer,
        )
        .unwrap();
        let server_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x67; 32]),
            &server_key,
            0,
            &authority,
            &issuer,
        )
        .unwrap();
        let client_trust = establish_trust(&client_credential, &authority, &issuer, [0x68; 32]);
        let server_trust = establish_trust(&server_credential, &authority, &issuer, [0x69; 32]);

        Self {
            authority,
            issuer,
            client_key,
            server_key,
            client_credential,
            server_credential,
            client_trust,
            server_trust,
        }
    }
}

#[tokio::test]
async fn trusted_session_endpoint_uses_provider_signers_and_fresh_session_ids() {
    let fixture = Fixture::new();
    let server =
        TrustedSessionQuicServer::bind(localhost_ephemeral(), QuicTransportConfig::default())
            .unwrap();
    let client =
        TrustedSessionQuicClient::bind(localhost_ephemeral(), QuicTransportConfig::default())
            .unwrap();
    let server_addr = server.local_addr().unwrap();

    let first = connect_pair(&fixture, &client, &server, server_addr).await;
    let first_id = first.0.session().context().unwrap().session_id();
    assert_eq!(
        first_id,
        first.1.session().context().unwrap().session_id()
    );
    first.0.transport().shutdown().await;
    first.1.transport().shutdown().await;

    let second = connect_pair(&fixture, &client, &server, server_addr).await;
    let second_id = second.0.session().context().unwrap().session_id();
    assert_ne!(first_id, second_id);
    assert_eq!(
        second_id,
        second.1.session().context().unwrap().session_id()
    );
    second.0.transport().shutdown().await;
    second.1.transport().shutdown().await;
}

async fn connect_pair(
    fixture: &Fixture,
    client: &TrustedSessionQuicClient,
    server: &TrustedSessionQuicServer,
    server_addr: SocketAddr,
) -> (
    crosslab_transport_quic::AuthenticatedQuicSession,
    crosslab_transport_quic::AuthenticatedQuicSession,
) {
    let client_signer = ProviderSigner(&fixture.client_key);
    let server_signer = ProviderSigner(&fixture.server_key);
    let server_trusts = [fixture.client_trust];
    let client_auth = QuicSessionAuthConfig::new(
        &fixture.authority,
        fixture.client_credential,
        &client_signer,
        &fixture.server_trust,
        protocol_ranges(),
        features(),
    );
    let server_auth = QuicSessionAuthConfig::new_with_peer_trusts(
        &fixture.authority,
        fixture.server_credential,
        &server_signer,
        &server_trusts,
        protocol_ranges(),
        features(),
    );

    let (client_result, server_result) = tokio::join!(
        client.connect_authenticated(server_addr, &client_auth, timeouts()),
        server.accept_authenticated(&server_auth, timeouts()),
    );

    (
        client_result.expect("trusted client should authenticate"),
        server_result.expect("trusted server should authenticate"),
    )
}

fn establish_trust(
    credential: &DeviceCredential,
    authority: &OwnerAuthorityState,
    issuer: &SigningKey,
    transition_id: [u8; 32],
) -> TrustRecord {
    PairingTrustTransition::issue(
        credential,
        TransitionId::from_bytes(transition_id),
        [transition_id[0].wrapping_add(1); 32],
        authority,
        issuer,
    )
    .unwrap()
    .establish(credential, authority)
    .unwrap()
}

fn protocol_ranges() -> Vec<ProtocolRange> {
    vec![ProtocolRange::new(1, 0, 2).unwrap()]
}

fn features() -> FeatureSet {
    FeatureSet::new(&[2], &[]).unwrap()
}

fn timeouts() -> QuicSessionTimeouts {
    QuicSessionTimeouts::new(WAIT, WAIT)
}

fn localhost_ephemeral() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}
