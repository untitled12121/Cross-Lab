use crosslab_m9_networking::candidate::endpoint::{
    M9_ALPN, direct_pair, unconnected_direct_endpoints,
};

#[tokio::test]
async fn direct_pair_has_matching_exporter() {
    let pair = direct_pair().await.expect("direct Iroh pair");

    assert_eq!(pair.client_binding(), pair.server_binding());
    assert_eq!(pair.client_binding().profile_id(), "quic-tls-exporter-v1");
    assert_eq!(pair.client_binding().bytes().len(), 32);

    pair.shutdown().await;
}

#[tokio::test]
async fn reconnect_changes_exporter() {
    let first = direct_pair().await.expect("first direct Iroh pair");
    let old = first.client_binding().bytes().to_vec();
    first.shutdown().await;

    let second = direct_pair().await.expect("second direct Iroh pair");
    assert_ne!(old, second.client_binding().bytes());
    second.shutdown().await;
}

#[tokio::test]
async fn minimal_endpoint_requires_explicit_address_data() {
    let pair = unconnected_direct_endpoints()
        .await
        .expect("minimal Iroh endpoints");

    assert!(pair.client().connect(pair.server().id(), M9_ALPN).await.is_err());

    pair.shutdown().await;
}
