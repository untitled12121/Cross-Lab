use crosslab_core::{SessionState, TransportSecurityClass};
use crosslab_identity::DeviceId;
use crosslab_policy::{NetworkClass, TrustState};
use crosslab_protocol::ProtocolVersion;
use crosslab_runtime::ConnectivityState;

use super::{
    ConnectivityDisplay, DevicePresentation, NetworkDisplay, SecurityDisplay, SessionDisplay,
    StatusFields, TrustDisplay,
};

#[test]
fn maps_runtime_status_into_presentation_safe_state() {
    let state = DevicePresentation::from_fields(StatusFields {
        peer_device_id: Some(DeviceId::from_bytes([0xab; 32])),
        trust_state: TrustState::Trusted,
        connectivity: ConnectivityState::Connected,
        session_state: SessionState::Active,
        protocol_version: Some(ProtocolVersion::new(1, 4)),
        network_class: NetworkClass::Local,
        security_class: TransportSecurityClass::AuthenticatedConfidentialChannel,
        metered: Some(false),
        capability_count: 2,
    });

    assert_eq!(state.peer_id(), Some("abababababababab"));
    assert_eq!(state.trust(), TrustDisplay::Trusted);
    assert_eq!(state.connectivity(), ConnectivityDisplay::Connected);
    assert_eq!(state.session(), SessionDisplay::Active);
    assert_eq!(state.protocol(), Some("1.4"));
    assert_eq!(state.network(), NetworkDisplay::Local);
    assert_eq!(state.security(), SecurityDisplay::Authenticated);
    assert_eq!(state.metered(), Some(false));
    assert_eq!(state.capability_count(), 2);
}

#[test]
fn maps_fail_closed_runtime_states_without_secret_material() {
    let state = DevicePresentation::from_fields(StatusFields {
        peer_device_id: Some(DeviceId::from_bytes([0xcd; 32])),
        trust_state: TrustState::Revoked,
        connectivity: ConnectivityState::Disconnected,
        session_state: SessionState::Revoked,
        protocol_version: None,
        network_class: NetworkClass::Remote,
        security_class: TransportSecurityClass::InProcessTest,
        metered: None,
        capability_count: 0,
    });

    assert_eq!(state.trust(), TrustDisplay::Revoked);
    assert_eq!(state.connectivity(), ConnectivityDisplay::Disconnected);
    assert_eq!(state.session(), SessionDisplay::Revoked);
    assert_eq!(state.network(), NetworkDisplay::Remote);
    assert_eq!(state.security(), SecurityDisplay::TestOnly);

    let debug = format!("{state:?}").to_ascii_lowercase();
    for forbidden in ["session_id", "channel_binding", "credential", "private_key", "secret"] {
        assert!(!debug.contains(forbidden), "presentation leaked {forbidden}");
    }
}
