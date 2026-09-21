use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use crosslab_crypto::SigningKey;
use crosslab_identity::{DeviceId, OwnerId};
use crosslab_protocol::{
    PairingHello, PairingRole, ProductPairingAck, ProductPairingAckKind, ProductPairingMessage,
};
use crosslab_transport_quic::{
    ProductPairingQuicClient, ProductPairingQuicError, ProductPairingQuicServer,
    ProductPairingQuicTimeouts,
};

fn timeouts() -> ProductPairingQuicTimeouts {
    ProductPairingQuicTimeouts::new(Duration::from_secs(3), Duration::from_secs(3))
}

fn hello() -> PairingHello {
    let key = SigningKey::from_secret_bytes([0x44; 32]);
    PairingHello::new(
        PairingRole::Joiner,
        1,
        [0x11; 16],
        OwnerId::from_bytes([0x22; 32]),
        DeviceId::from_bytes([0x33; 32]),
        key.verifying_key(),
        [0x55; 32],
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn provisional_pairing_channel_round_trips_bounded_messages() {
    let server = ProductPairingQuicServer::bind(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        timeouts(),
    )
    .unwrap();
    let remote = server.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let mut channel = server.accept().await.unwrap();
        assert_eq!(
            channel.receive().await.unwrap(),
            ProductPairingMessage::Hello(hello())
        );
        channel
            .send(&ProductPairingMessage::Ack(ProductPairingAck::new(
                [0x11; 16],
                PairingRole::Inviter,
                ProductPairingAckKind::Complete,
            )))
            .await
            .unwrap();
    });

    let mut client = ProductPairingQuicClient::connect(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        remote,
        timeouts(),
    )
    .await
    .unwrap();
    client
        .send(&ProductPairingMessage::Hello(hello()))
        .await
        .unwrap();
    assert_eq!(
        client.receive().await.unwrap(),
        ProductPairingMessage::Ack(ProductPairingAck::new(
            [0x11; 16],
            PairingRole::Inviter,
            ProductPairingAckKind::Complete,
        ))
    );

    server_task.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pairing_listener_allows_only_one_accept_operation() {
    let server = ProductPairingQuicServer::bind(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
        ProductPairingQuicTimeouts::new(Duration::from_millis(50), Duration::from_secs(1)),
    )
    .unwrap();

    assert!(matches!(
        server.accept().await,
        Err(ProductPairingQuicError::Timeout)
    ));

    // A timed-out accept is retryable because no joiner was bound to the invitation.
    assert!(matches!(
        server.accept().await,
        Err(ProductPairingQuicError::Timeout)
    ));
}
