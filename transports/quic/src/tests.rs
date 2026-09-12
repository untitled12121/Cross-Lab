use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use quinn::{ClientConfig, Connection, Endpoint, ServerConfig};
use rustls::{RootCertStore, pki_types::PrivatePkcs8KeyDer};

use crate::binding::derive_channel_binding;

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
