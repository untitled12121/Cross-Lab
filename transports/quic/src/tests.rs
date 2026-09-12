use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use crosslab_core::{
    ConnectionMetadata, ControlReceiveError, ControlSendError, TransportConnection,
    TransportSecurityClass,
};
use quinn::{ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig};
use rustls::{RootCertStore, pki_types::PrivatePkcs8KeyDer};
use tokio::time::timeout;

use crate::{
    binding::derive_channel_binding,
    config::QuicTransportConfig,
    connection::QuicTransportConnection,
    record::{RecordError, read_record, write_record},
};

struct LoopbackConnectionPair {
    _client_endpoint: Endpoint,
    _server_endpoint: Endpoint,
    client: Connection,
    server: Connection,
}

struct LoopbackTransportPair {
    _client_endpoint: Endpoint,
    _server_endpoint: Endpoint,
    client: QuicTransportConnection,
    server: QuicTransportConnection,
}

#[tokio::test]
async fn exporter_binding_matches_peer_and_changes_on_reconnect() {
    let first = loopback_connection_pair().await;
    let first_client = derive_channel_binding(&first.client).unwrap();
    let first_server = derive_channel_binding(&first.server).unwrap();

    assert_eq!(first_client, first_server);
    assert_eq!(first_client.profile_id(), "quic-tls-exporter-v1");
    assert_eq!(first_client.bytes().len(), 32);

    let second = loopback_connection_pair().await;
    let second_client = derive_channel_binding(&second.client).unwrap();
    assert_ne!(first_client.bytes(), second_client.bytes());
}

#[tokio::test]
async fn record_round_trip_accepts_exact_limit() {
    let pair = loopback_connection_pair().await;
    let mut send = open_test_bi_send(&pair).await;
    let payload = vec![0x41; 8];

    write_record(&mut send, &payload, 8).await.unwrap();
    send.finish().unwrap();
    let mut recv = accept_test_bi_recv(&pair).await;

    assert_eq!(read_record(&mut recv, 8, false).await.unwrap(), payload);
}

#[tokio::test]
async fn record_reader_rejects_declared_length_before_allocating_body() {
    let pair = loopback_connection_pair().await;
    let mut send = open_test_bi_send(&pair).await;
    send.write_all(&9_u32.to_be_bytes()).await.unwrap();
    send.write_all(&[0_u8]).await.unwrap();
    send.finish().unwrap();
    let mut recv = accept_test_bi_recv(&pair).await;

    assert_eq!(
        read_record(&mut recv, 8, false).await,
        Err(RecordError::TooLarge {
            declared: 9,
            max: 8,
        })
    );
}

#[tokio::test]
async fn record_reader_rejects_empty_when_not_allowed() {
    let pair = loopback_connection_pair().await;
    let mut send = open_test_bi_send(&pair).await;
    send.write_all(&0_u32.to_be_bytes()).await.unwrap();
    send.finish().unwrap();
    let mut recv = accept_test_bi_recv(&pair).await;

    assert_eq!(
        read_record(&mut recv, 8, false).await,
        Err(RecordError::Empty)
    );
}

#[tokio::test]
async fn record_reader_rejects_truncated_body() {
    let pair = loopback_connection_pair().await;
    let mut send = open_test_bi_send(&pair).await;
    send.write_all(&4_u32.to_be_bytes()).await.unwrap();
    send.write_all(&[0x51, 0x52]).await.unwrap();
    send.finish().unwrap();
    let mut recv = accept_test_bi_recv(&pair).await;

    assert!(matches!(
        read_record(&mut recv, 8, false).await,
        Err(RecordError::Read)
    ));
}

#[tokio::test]
async fn control_bridge_preserves_order_and_bounded_backpressure() {
    let pair = promoted_loopback_transport_pair(1, 8).await;

    assert_eq!(
        pair.client.security_class(),
        TransportSecurityClass::AuthenticatedConfidentialChannel
    );
    assert_eq!(
        pair.client.try_receive_control(),
        Err(ControlReceiveError::Empty)
    );

    pair.client.try_send_control(vec![1]).unwrap();
    assert_eq!(
        pair.client.try_send_control(vec![2]),
        Err(ControlSendError::Full(vec![2]))
    );

    assert_eq!(eventually_receive_control(&pair.server).await, vec![1]);
    pair.client.try_send_control(vec![2]).unwrap();
    assert_eq!(eventually_receive_control(&pair.server).await, vec![2]);

    pair.client.shutdown().await;
    pair.server.shutdown().await;
}

#[tokio::test]
async fn control_bridge_rejects_oversize_without_consuming_queue_capacity() {
    let pair = promoted_loopback_transport_pair(1, 1).await;

    let oversized = vec![0x61, 0x62];
    assert_eq!(
        pair.client.try_send_control(oversized.clone()),
        Err(ControlSendError::TooLarge(oversized))
    );

    pair.client.try_send_control(vec![0x63]).unwrap();
    assert_eq!(eventually_receive_control(&pair.server).await, vec![0x63]);

    pair.client.shutdown().await;
    pair.server.shutdown().await;
}

#[tokio::test]
async fn control_bridge_remote_close_becomes_terminal() {
    let pair = promoted_loopback_transport_pair(2, 8).await;

    pair.server.close();
    eventually_closed(&pair.client).await;

    let frame = vec![0x71];
    assert_eq!(
        pair.client.try_send_control(frame.clone()),
        Err(ControlSendError::Closed(frame))
    );
    assert_eq!(
        pair.client.try_receive_control(),
        Err(ControlReceiveError::Closed)
    );

    pair.client.shutdown().await;
    pair.server.shutdown().await;
}

#[tokio::test]
async fn shutdown_closes_connection_and_owned_tasks() {
    let pair = promoted_loopback_transport_pair(2, 8).await;

    pair.client.shutdown().await;
    assert!(pair.client.is_closed());
    assert_eq!(
        pair.client.try_send_control(vec![0x81]),
        Err(ControlSendError::Closed(vec![0x81]))
    );

    eventually_closed(&pair.server).await;
    pair.server.shutdown().await;
}

async fn promoted_loopback_transport_pair(
    control_capacity: usize,
    max_control_frame_bytes: usize,
) -> LoopbackTransportPair {
    let raw = loopback_connection_pair().await;
    let client_binding = derive_channel_binding(&raw.client).unwrap();
    let server_binding = derive_channel_binding(&raw.server).unwrap();
    let client_local = raw._client_endpoint.local_addr().unwrap();
    let server_local = raw._server_endpoint.local_addr().unwrap();

    let (mut client_send, client_recv) = raw.client.open_bi().await.unwrap();
    write_record(&mut client_send, &[0xA5], 1).await.unwrap();
    let (server_send, mut server_recv) = raw.server.accept_bi().await.unwrap();
    assert_eq!(
        read_record(&mut server_recv, 1, false).await.unwrap(),
        vec![0xA5]
    );

    let config = QuicTransportConfig::default().with_control_limits(
        nonzero(control_capacity),
        nonzero(max_control_frame_bytes),
    );
    let client = QuicTransportConnection::new(
        raw.client.clone(),
        client_send,
        client_recv,
        client_binding,
        ConnectionMetadata::new(
            Some(client_local.to_string()),
            Some(server_local.to_string()),
            Some(false),
        ),
        config,
    );
    let server = QuicTransportConnection::new(
        raw.server.clone(),
        server_send,
        server_recv,
        server_binding,
        ConnectionMetadata::new(
            Some(server_local.to_string()),
            Some(client_local.to_string()),
            Some(false),
        ),
        config,
    );

    LoopbackTransportPair {
        _client_endpoint: raw._client_endpoint,
        _server_endpoint: raw._server_endpoint,
        client,
        server,
    }
}

async fn eventually_receive_control(connection: &QuicTransportConnection) -> Vec<u8> {
    timeout(Duration::from_secs(2), async {
        loop {
            match connection.try_receive_control() {
                Ok(frame) => return frame,
                Err(ControlReceiveError::Empty) => tokio::task::yield_now().await,
                Err(ControlReceiveError::Closed) => {
                    panic!("control bridge closed before delivering the expected frame")
                }
            }
        }
    })
    .await
    .expect("control frame was not delivered before timeout")
}

async fn eventually_closed(connection: &QuicTransportConnection) {
    timeout(Duration::from_secs(2), async {
        while !connection.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("transport did not become terminal before timeout");
}

async fn open_test_bi_send(pair: &LoopbackConnectionPair) -> SendStream {
    let (send, _) = pair.client.open_bi().await.unwrap();
    send
}

async fn accept_test_bi_recv(pair: &LoopbackConnectionPair) -> RecvStream {
    let (_, recv) = pair.server.accept_bi().await.unwrap();
    recv
}

async fn loopback_connection_pair() -> LoopbackConnectionPair {
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

    LoopbackConnectionPair {
        _client_endpoint: client_endpoint,
        _server_endpoint: server_endpoint,
        client: client.unwrap(),
        server: server.unwrap(),
    }
}

fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
