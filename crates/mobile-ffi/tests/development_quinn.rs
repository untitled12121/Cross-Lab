#![cfg(feature = "development-provisioning")]

use std::{
    fs,
    net::SocketAddr,
    path::PathBuf,
    time::Duration,
};

use crosslab_crypto::SigningKey;
use crosslab_mobile_ffi::{
    MobileConnectivityState, MobileRuntime, MobileRuntimeSnapshot, MobileSessionState,
    MobileTransportSecurity, MobileTrustState,
};
use crosslab_transport_quic::{
    QuicSessionTimeouts,
    development::DevelopmentProvisioning,
};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(3);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mobile_facade_tracks_authenticated_quinn_disconnect_and_fresh_reconnect() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()].unwrap();
    let certificate_der = certified.cert.der().as_ref().to_vec();
    let private_key_der = certified.signing_key.serialize_der();

    let client_key = SigningKey::from_secret_bytes([0x31; 32]);
    let server_key = SigningKey::from_secret_bytes([0x32; 32]);

    let server = DevelopmentProvisioning::from_json(
        provisioning_document(
            [0x12; 32],
            [0x21; 32],
            [0x22; 32],
            [0x42; 32],
            [0x32; 32],
            [0x41; 32],
            server_key.verifying_key().to_bytes(),
            client_key.verifying_key().to_bytes(),
            json!({
                "role": "server",
                "bind_addr": "127.0.0.1:0",
                "certificate_chain_der_hex": [hex(&certificate_der)],
                "private_key_pkcs8_der_hex": hex(&private_key_der),
            }),
        )
        .to_string()
        .as_bytes(),
    )
    .unwrap()
    .into_server()
    .unwrap();
    let server_addr = server.local_addr().unwrap();

    let client_path = temp_provisioning_path();
    fs::write(
        &client_path,
        provisioning_document(
            [0x12; 32],
            [0x21; 32],
            [0x22; 32],
            [0x41; 32],
            [0x31; 32],
            [0x42; 32],
            client_key.verifying_key().to_bytes(),
            server_key.verifying_key().to_bytes(),
            json!({
                "role": "client",
                "bind_addr": "127.0.0.1:0",
                "remote_addr": server_addr.to_string(),
                "server_name": "localhost",
                "trusted_server_certificate_der_hex": [hex(&certificate_der)],
            }),
        )
        .to_string(),
    )
    .unwrap();

    let mobile = MobileRuntime::new();
    mobile.start().unwrap();
    mobile
        .configure_development_client(client_path.to_string_lossy().into_owned())
        .unwrap();

    let first_server = server.accept_authenticated(timeouts()).await.unwrap();
    let first = wait_for(&mobile, |snapshot| {
        snapshot.connectivity == MobileConnectivityState::Connected
            && snapshot.session == MobileSessionState::Active
    });

    let first_server_context = first_server.session().context().unwrap();
    assert_eq!(
        first.peer_device_id.as_deref(),
        Some(&hex(first_server_context.local_device_id().as_bytes()))
    );
    assert_eq!(
        first.local_device_id.as_deref(),
        Some(&hex(first_server_context.peer_device_id().as_bytes()))
    );
    assert_eq!(
        first.session_id.as_deref(),
        Some(&hex(&first_server_context.session_id().to_bytes()))
    );
    assert_eq!(first.trust, MobileTrustState::Trusted);
    assert_eq!(
        first.transport_security,
        MobileTransportSecurity::Authenticated
    );

    mobile.network_lost().unwrap();
    let disconnected = wait_for(&mobile, |snapshot| {
        snapshot.connectivity == MobileConnectivityState::Disconnected
            && snapshot.session != MobileSessionState::Active
    });
    assert!(disconnected.session_id.is_none());

    mobile.network_available().unwrap();
    let second_server = server.accept_authenticated(timeouts()).await.unwrap();
    let second = wait_for(&mobile, |snapshot| {
        snapshot.connectivity == MobileConnectivityState::Connected
            && snapshot.session == MobileSessionState::Active
            && snapshot.session_id != first.session_id
    });

    let second_server_context = second_server.session().context().unwrap();
    assert_eq!(
        second.session_id.as_deref(),
        Some(&hex(&second_server_context.session_id().to_bytes()))
    );
    assert_ne!(second.session_id, first.session_id);

    first_server.transport().shutdown().await;
    second_server.transport().shutdown().await;
    mobile.stop().unwrap();
    let _ = fs::remove_file(client_path);
}

fn wait_for(
    runtime: &MobileRuntime,
    predicate: impl Fn(&MobileRuntimeSnapshot) -> bool,
) -> MobileRuntimeSnapshot {
    for _ in 0..8 {
        if let Some(event) = runtime.wait_event(1_000).unwrap()
            && predicate(&event.snapshot)
        {
            return event.snapshot;
        }
    }
    panic!("expected mobile runtime state was not published");
}

#[allow(clippy::too_many_arguments)]
fn provisioning_document(
    owner_id: [u8; 32],
    root_secret: [u8; 32],
    issuer_secret: [u8; 32],
    local_device_id: [u8; 32],
    local_device_secret: [u8; 32],
    peer_device_id: [u8; 32],
    local_public_key: [u8; 32],
    peer_public_key: [u8; 32],
    endpoint: serde_json::Value,
) -> serde_json::Value {
    assert_eq!(
        local_public_key,
        SigningKey::from_secret_bytes(local_device_secret)
            .verifying_key()
            .to_bytes()
    );

    json!({
        "identity": {
            "owner_id_hex": hex(&owner_id),
            "owner_root_secret_hex": hex(&root_secret),
            "owner_root_epoch": 0,
            "device_signing_secret_hex": hex(&issuer_secret),
            "device_signing_epoch": 0,
            "local_device_id_hex": hex(&local_device_id),
            "local_device_secret_hex": hex(&local_device_secret),
            "local_credential_epoch": 0,
            "peer_device_id_hex": hex(&peer_device_id),
            "peer_device_public_key_hex": hex(&peer_public_key),
            "peer_credential_epoch": 0
        },
        "protocol": {
            "ranges": [{
                "major": 1,
                "min_minor": 0,
                "max_minor": 2
            }],
            "supported_features": [2],
            "required_features": []
        },
        "endpoint": endpoint
    })
}

fn temp_provisioning_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "crosslab-mobile-development-{}.json",
        std::process::id()
    ))
}

fn timeouts() -> QuicSessionTimeouts {
    QuicSessionTimeouts::new(WAIT, WAIT)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
