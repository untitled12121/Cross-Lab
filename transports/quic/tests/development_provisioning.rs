#![cfg(feature = "development-provisioning")]

use std::time::Duration;

use crosslab_core::SessionError;
use crosslab_crypto::SigningKey;
use crosslab_policy::TrustState;
use crosslab_transport_quic::{
    QuicSessionError, QuicSessionTimeouts,
    development::{DevelopmentProvisioning, DevelopmentProvisioningError},
};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(2);

#[tokio::test]
async fn explicit_development_files_establish_authenticated_quinn_session() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let certificate_der = certified.cert.der().as_ref().to_vec();
    let private_key_der = certified.signing_key.serialize_der();

    let client_key = SigningKey::from_secret_bytes([0x31; 32]);
    let server_key = SigningKey::from_secret_bytes([0x32; 32]);

    let server_document = provisioning_document(
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
    );
    let server = DevelopmentProvisioning::from_json(server_document.to_string().as_bytes())
        .unwrap()
        .into_server()
        .unwrap();
    let server_addr = server.local_addr().unwrap();

    let client_document = provisioning_document(
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
    );
    let client = DevelopmentProvisioning::from_json(client_document.to_string().as_bytes())
        .unwrap()
        .into_client()
        .unwrap();

    let timeouts = QuicSessionTimeouts::new(WAIT, WAIT);
    let (client_session, server_session) = tokio::join!(
        client.connect_authenticated(timeouts),
        server.accept_authenticated(timeouts),
    );
    let client_session = client_session.unwrap();
    let server_session = server_session.unwrap();

    let client_context = client_session.session().context().unwrap();
    let server_context = server_session.session().context().unwrap();
    assert_eq!(client_context.session_id(), server_context.session_id());
    assert_eq!(
        client_context.peer_device_id(),
        server_context.local_device_id()
    );
    assert_eq!(
        server_context.peer_device_id(),
        client_context.local_device_id()
    );

    client_session.transport().shutdown().await;
    server_session.transport().shutdown().await;
}

#[tokio::test]
async fn development_server_revocation_rejects_fresh_reconnect() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let certificate_der = certified.cert.der().as_ref().to_vec();
    let private_key_der = certified.signing_key.serialize_der();

    let client_key = SigningKey::from_secret_bytes([0x51; 32]);
    let server_key = SigningKey::from_secret_bytes([0x52; 32]);

    let server_document = provisioning_document(
        [0x62; 32],
        [0x63; 32],
        [0x64; 32],
        [0x72; 32],
        [0x52; 32],
        [0x71; 32],
        server_key.verifying_key().to_bytes(),
        client_key.verifying_key().to_bytes(),
        json!({
            "role": "server",
            "bind_addr": "127.0.0.1:0",
            "certificate_chain_der_hex": [hex(&certificate_der)],
            "private_key_pkcs8_der_hex": hex(&private_key_der),
        }),
    );
    let mut server = DevelopmentProvisioning::from_json(server_document.to_string().as_bytes())
        .unwrap()
        .into_server()
        .unwrap();
    let server_addr = server.local_addr().unwrap();

    let client_document = provisioning_document(
        [0x62; 32],
        [0x63; 32],
        [0x64; 32],
        [0x71; 32],
        [0x51; 32],
        [0x72; 32],
        client_key.verifying_key().to_bytes(),
        server_key.verifying_key().to_bytes(),
        json!({
            "role": "client",
            "bind_addr": "127.0.0.1:0",
            "remote_addr": server_addr.to_string(),
            "server_name": "localhost",
            "trusted_server_certificate_der_hex": [hex(&certificate_der)],
        }),
    );
    let client = DevelopmentProvisioning::from_json(client_document.to_string().as_bytes())
        .unwrap()
        .into_client()
        .unwrap();

    let timeouts = QuicSessionTimeouts::new(WAIT, WAIT);
    let (client_session, server_session) = tokio::join!(
        client.connect_authenticated(timeouts),
        server.accept_authenticated(timeouts),
    );
    let client_session = client_session.unwrap();
    let server_session = server_session.unwrap();

    let revoked = server.revoke_peer_trust().unwrap();
    assert_eq!(revoked.state(), TrustState::Revoked);

    client_session.transport().shutdown().await;
    server_session.transport().shutdown().await;

    let (client_result, server_result) = tokio::join!(
        client.connect_authenticated(timeouts),
        server.accept_authenticated(timeouts),
    );
    assert!(matches!(
        server_result,
        Err(QuicSessionError::Session(SessionError::PeerNotTrusted))
    ));
    if let Ok(session) = client_result {
        session.transport().shutdown().await;
    }
}

#[test]
fn development_provisioning_rejects_wrong_endpoint_role() {
    let local_key = SigningKey::from_secret_bytes([0x31; 32]);
    let peer_key = SigningKey::from_secret_bytes([0x32; 32]);
    let document = provisioning_document(
        [0x12; 32],
        [0x21; 32],
        [0x22; 32],
        [0x41; 32],
        [0x31; 32],
        [0x42; 32],
        local_key.verifying_key().to_bytes(),
        peer_key.verifying_key().to_bytes(),
        json!({
            "role": "server",
            "bind_addr": "127.0.0.1:0",
            "certificate_chain_der_hex": ["00"],
            "private_key_pkcs8_der_hex": "00",
        }),
    );

    let result = DevelopmentProvisioning::from_json(document.to_string().as_bytes())
        .unwrap()
        .into_client();

    assert!(matches!(
        result,
        Err(DevelopmentProvisioningError::WrongRole)
    ));
}

#[cfg(unix)]
#[test]
fn development_provisioning_rejects_group_readable_files() {
    use std::{fs, os::unix::fs::PermissionsExt as _};

    let path = std::env::temp_dir().join(format!(
        "crosslab-quic-development-permissions-{}.json",
        std::process::id()
    ));
    fs::write(&path, b"{}").unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o644);
    fs::set_permissions(&path, permissions).unwrap();

    let result = DevelopmentProvisioning::from_path(&path);
    let _ = fs::remove_file(path);

    assert!(matches!(
        result,
        Err(DevelopmentProvisioningError::InsecurePermissions)
    ));
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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
