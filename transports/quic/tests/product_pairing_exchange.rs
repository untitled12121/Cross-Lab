use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use crosslab_core::{PairingBootstrap, PairingInstant, PairingInvitation, PairingSecret};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_runtime::{
    ProductPairingExchangeState, ProductPairingInviter, ProductPairingInviterExchange,
    ProductPairingJoiner, ProductPairingJoinerExchange,
};
use crosslab_transport_quic::{
    ProductPairingQuicClient, ProductPairingQuicServer, ProductPairingQuicTimeouts,
};

fn timeouts() -> ProductPairingQuicTimeouts {
    ProductPairingQuicTimeouts::new(Duration::from_secs(3), Duration::from_secs(3))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_product_pairing_coordinator_runs_over_provisional_quic() {
    let owner_id = OwnerId::from_bytes([0x31; 32]);
    let root_key = SigningKey::from_secret_bytes([0x32; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer = SigningKey::from_secret_bytes([0x33; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer,
        0,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();

    let inviter_key = SigningKey::from_secret_bytes([0x34; 32]);
    let inviter_id = DeviceId::from_bytes([0x35; 32]);
    let inviter_credential =
        DeviceCredential::issue(owner_id, inviter_id, &inviter_key, 0, &authority, &issuer)
            .unwrap();
    let pairing_id = crosslab_core::PairingId::from_bytes([0x36; 16]);
    let invitation = PairingInvitation::from_parts(
        pairing_id,
        PairingSecret::from_bytes([0x37; 32]),
        owner_id,
        inviter_id,
        PairingInstant::from_ticks(0),
        PairingInstant::from_ticks(100),
    )
    .unwrap();
    let mut qr_invitation = PairingInvitation::from_parts(
        pairing_id,
        PairingSecret::from_bytes([0x37; 32]),
        owner_id,
        inviter_id,
        PairingInstant::from_ticks(0),
        PairingInstant::from_ticks(100),
    )
    .unwrap();
    let code = qr_invitation
        .bootstrap_code_at(PairingInstant::from_ticks(0))
        .unwrap();
    let bootstrap = PairingBootstrap::decode(code.as_str()).unwrap();
    drop(code);

    let server = ProductPairingQuicServer::bind(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        timeouts(),
    )
    .unwrap();
    let remote = server.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let pairing =
            ProductPairingInviter::new(invitation, inviter_credential, &authority).unwrap();
        let mut exchange = ProductPairingInviterExchange::new(pairing);
        let mut channel = server.accept().await.unwrap();

        let hello = channel.receive().await.unwrap();
        let hello = exchange
            .accept_joiner_hello(hello, PairingInstant::from_ticks(10))
            .unwrap();
        channel.send(&hello).await.unwrap();

        let confirmation = channel.receive().await.unwrap();
        let confirmation = exchange
            .accept_joiner_confirmation(confirmation, PairingInstant::from_ticks(20))
            .unwrap();
        channel.send(&confirmation).await.unwrap();

        let credential = exchange
            .issue_credential_bundle(&authority, &issuer, PairingInstant::from_ticks(30))
            .unwrap();
        channel.send(&credential).await.unwrap();

        let proof = channel.receive().await.unwrap();
        let commit = exchange
            .accept_credential_proof(proof, &authority, &issuer, PairingInstant::from_ticks(40))
            .unwrap();
        assert_eq!(commit.peer_credential().owner_id(), owner_id);

        let trust = exchange.local_persisted().unwrap();
        channel.send(&trust).await.unwrap();

        let persisted = channel.receive().await.unwrap();
        let complete = exchange.accept_peer_persisted(persisted).unwrap();
        channel.send(&complete).await.unwrap();
        assert_eq!(exchange.state(), ProductPairingExchangeState::Complete);
        channel.finish().await.unwrap();
    });

    let joiner_key = SigningKey::from_secret_bytes([0x38; 32]);
    let joiner_id = DeviceId::from_bytes([0x39; 32]);
    let pairing = ProductPairingJoiner::new(bootstrap, joiner_id, &joiner_key).unwrap();
    let mut exchange = ProductPairingJoinerExchange::new(pairing);
    let mut channel = ProductPairingQuicClient::connect(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        remote,
        timeouts(),
    )
    .await
    .unwrap();

    channel.send(&exchange.hello()).await.unwrap();

    let hello = channel.receive().await.unwrap();
    let confirmation = exchange.accept_inviter_hello(hello).unwrap();
    channel.send(&confirmation).await.unwrap();

    let confirmation = channel.receive().await.unwrap();
    exchange.accept_inviter_confirmation(confirmation).unwrap();

    let credential = channel.receive().await.unwrap();
    let proof = exchange
        .accept_credential_bundle(credential, &joiner_key)
        .unwrap();
    channel.send(&proof).await.unwrap();

    let trust = channel.receive().await.unwrap();
    let completion = exchange.accept_trust_bundle(trust).unwrap();
    assert_eq!(completion.local_credential().device_id(), joiner_id);

    let persisted = exchange.local_persisted().unwrap();
    channel.send(&persisted).await.unwrap();

    let complete = channel.receive().await.unwrap();
    exchange.accept_complete(complete).unwrap();
    assert_eq!(exchange.state(), ProductPairingExchangeState::Complete);

    server_task.await.unwrap();
}
