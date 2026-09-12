use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use quinn::{ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig};
use rustls::{RootCertStore, pki_types::PrivatePkcs8KeyDer};

use crate::{
    binding::derive_channel_binding,
    record::{RecordError, read_record, write_record},
};

struct LoopbackConnectionPair {
    _client_endpoint: Endpoint,
    _server_endpoint: Endpoint,
    client: Connection,
    server: Connection,
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
    let (mut send, mut recv) = open_test_bi(&pair).await;
    let payload = vec![0x41; 8];

    write_record(&mut send, &payload, 8).await.unwrap();
    send.finish().unwrap();

    assert_eq!(read_record(&mut recv, 8, false).await.unwrap(), payload);
}

#[tokio::test]
async fn record_reader_rejects_declared_length_before_allocating_body() {
    let pair = loopback_connection_pair().await;
    let (mut send, mut recv) = open_test_bi(&pair).await;
    send.write_all(&9_u32.to_be_bytes()).await.unwrap();
    send.write_all(&[0_u8]).await.unwrap();
    send.finish().unwrap();

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
    let (mut send, mut recv) = open_test_bi(&pair).await;
    send.write_all(&0_u32.to_be_bytes()).await.unwrap();
    send.finish().unwrap();

    assert_eq!(
        read_record(&mut recv, 8, false).await,
        Err(RecordError::Empty)
    );
}

#[tokio::test]
async fn record_reader_rejects_truncated_body() {
    let pair = loopback_connection_pair().await;
    let (mut send, mut recv) = open_test_bi(&pair).await;
    send.write_all(&4_u32.to_be_bytes()).await.unwrap();
    send.write_all(&[0x51, 0x52]).await.unwrap();
    send.finish().unwrap();

    assert!(matches!(
        read_record(&mut recv, 8, false).await,
        Err(RecordError::Read)
    ));
}

async fn open_test_bi(pair: &LoopbackConnectionPair) -> (SendStream, RecvStream) {
    let (client, server) = tokio::join!(pair.client.open_bi(), pair.server.accept_bi());
    let (send, _) = client.unwrap();
    let (_, recv) = server.unwrap();
    (send, recv)
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
