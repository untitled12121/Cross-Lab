use crosslab_core::{SessionState, TransportSecurityClass};
use crosslab_identity::{DeviceId, OwnerId};
use crosslab_policy::{NetworkClass, TrustState};
use crosslab_protocol::ProtocolVersion;
use crosslab_runtime::ConnectivityState;

use super::{
    ConnectivityDisplay, DevicePresentation, DevicesFeatureState, NetworkDisplay,
    PresenceControlAction, PresenceDisplay, SecurityDisplay, SessionDisplay, StatusFields,
    TrustDisplay, VerifiedSessionDisplay,
};

#[test]
fn maps_runtime_status_into_presentation_safe_state() {
    let state = DevicePresentation::from_fields(StatusFields {
        owner_id: Some(OwnerId::from_bytes([0xaa; 32])),
        local_device_id: Some(DeviceId::from_bytes([0xa1; 32])),
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

    assert_eq!(state.owner_id(), Some("aaaaaaaaaaaaaaaa"));
    assert_eq!(state.local_device_id(), Some("a1a1a1a1a1a1a1a1"));
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
        owner_id: Some(OwnerId::from_bytes([0xcc; 32])),
        local_device_id: Some(DeviceId::from_bytes([0xc1; 32])),
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
    for forbidden in [
        "session_id",
        "channel_binding",
        "credential",
        "private_key",
        "secret",
    ] {
        assert!(
            !debug.contains(forbidden),
            "presentation leaked {forbidden}"
        );
    }
}

#[test]
fn saved_trust_never_implies_a_live_session() {
    let id = DeviceId::from_bytes([0xab; 32]);
    let saved = DevicesFeatureState::empty().saved_trusted_peers(&[id]);

    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].short_id(), "abababababababab");
    assert!(!saved[0].in_authenticated_session());
}

#[test]
fn saved_peers_require_full_id_and_authenticated_connection() {
    let current_id = DeviceId::from_bytes([0xab; 32]);
    let mut lookalike = [0xab; 32];
    lookalike[31] = 0xcd;
    let lookalike_id = DeviceId::from_bytes(lookalike);
    let presentation = DevicePresentation::from_fields(StatusFields {
        owner_id: None,
        local_device_id: None,
        peer_device_id: Some(current_id),
        trust_state: TrustState::Trusted,
        connectivity: ConnectivityState::Connected,
        session_state: SessionState::Active,
        protocol_version: None,
        network_class: NetworkClass::Local,
        security_class: TransportSecurityClass::AuthenticatedConfidentialChannel,
        metered: None,
        capability_count: 0,
    });
    let mut state = DevicesFeatureState::empty();
    state.current = Some(presentation);
    state.presence = PresenceDisplay::Online;

    let saved = state.saved_trusted_peers(&[lookalike_id, current_id]);
    assert!(!saved[0].in_authenticated_session());
    assert!(saved[1].in_authenticated_session());

    state.current.as_mut().unwrap().session = SessionDisplay::Closed;
    assert!(!state.saved_trusted_peers(&[current_id])[0].in_authenticated_session());
}

#[test]
fn native_capability_actions_require_current_authenticated_session() {
    let peer = DeviceId::from_bytes([0x74; 32]);
    let mut device = DevicePresentation::from_fields(StatusFields {
        owner_id: None,
        local_device_id: None,
        peer_device_id: Some(peer),
        trust_state: TrustState::Trusted,
        connectivity: ConnectivityState::Connected,
        session_state: SessionState::Active,
        protocol_version: None,
        network_class: NetworkClass::Local,
        security_class: TransportSecurityClass::AuthenticatedConfidentialChannel,
        metered: None,
        capability_count: 1,
    });
    device.capability_ids.push("files.transfer".to_owned());
    let mut state = DevicesFeatureState::empty();
    state.current = Some(device);
    state.presence = PresenceDisplay::Online;

    assert!(state.has_authenticated_session());
    assert!(state.has_authenticated_capability("files.transfer"));
    assert!(!state.has_authenticated_capability("clipboard.read"));

    state.presence = PresenceDisplay::Reconnecting;
    assert!(!state.has_authenticated_capability("files.transfer"));
    assert!(!state.saved_trusted_peers(&[peer])[0].in_authenticated_session());

    state.presence = PresenceDisplay::Online;
    state.current.as_mut().unwrap().security = SecurityDisplay::TestOnly;
    assert!(!state.has_authenticated_capability("files.transfer"));
    state.current.as_mut().unwrap().security = SecurityDisplay::Authenticated;
    state.current.as_mut().unwrap().trust = TrustDisplay::Revoked;
    assert!(!state.has_authenticated_capability("files.transfer"));
    state.current.as_mut().unwrap().trust = TrustDisplay::Trusted;
    state.current.as_mut().unwrap().session = SessionDisplay::Closed;
    assert!(!state.has_authenticated_capability("files.transfer"));
}

#[test]
fn old_session_results_are_rejected_after_reconnect_or_peer_change() {
    use super::AuthenticatedSessionBinding;
    use crosslab_policy::SessionId;

    let old = DeviceId::from_bytes([0x44; 32]);
    let newer = DeviceId::from_bytes([0x45; 32]);
    let first = AuthenticatedSessionBinding {
        peer: old,
        session: SessionId::from_bytes([0x71; 32]),
    };
    let mut device = DevicePresentation::from_fields(StatusFields {
        owner_id: None,
        local_device_id: None,
        peer_device_id: Some(old),
        trust_state: TrustState::Trusted,
        connectivity: ConnectivityState::Connected,
        session_state: SessionState::Active,
        protocol_version: None,
        network_class: NetworkClass::Local,
        security_class: TransportSecurityClass::AuthenticatedConfidentialChannel,
        metered: None,
        capability_count: 1,
    });
    device.capability_ids.push("clipboard.read".to_owned());
    let mut state = DevicesFeatureState::empty();
    state.presence = PresenceDisplay::Online;
    state.current = Some(device);
    state.session_binding = Some(first);
    assert!(state.matches_session_binding(first, "clipboard.read"));
    assert!(!state.matches_session_binding(first, "files.transfer"));
    assert!(!format!("{state:?}").contains("71717171"));

    state.session_binding = Some(AuthenticatedSessionBinding {
        peer: old,
        session: SessionId::from_bytes([0x72; 32]),
    });
    assert!(!state.matches_session_binding(first, "clipboard.read"));

    state.session_binding = Some(first);
    state.current.as_mut().unwrap().peer_device_id = Some(newer);
    assert!(!state.matches_session_binding(first, "clipboard.read"));

    state.current.as_mut().unwrap().peer_device_id = Some(old);
    state.presence = PresenceDisplay::Reconnecting;
    assert!(!state.matches_session_binding(first, "clipboard.read"));
    state.clear();
    assert!(state.current_session_binding("clipboard.read").is_none());
}

#[test]
fn saved_trusted_device_presentation_is_bounded() {
    let ids = (0..30)
        .map(|value| {
            let mut bytes = [0u8; 32];
            bytes[31] = value;
            DeviceId::from_bytes(bytes)
        })
        .collect::<Vec<_>>();
    let saved = DevicesFeatureState::empty().saved_trusted_peers(&ids);
    assert_eq!(saved.len(), 24);
}

#[test]
fn reconnect_action_is_offered_after_pause_and_failure() {
    use super::PresenceDisplay;

    assert!(PresenceDisplay::Paused.offers_reconnect());
    assert!(PresenceDisplay::Failed.offers_reconnect());
    assert!(PresenceDisplay::Unavailable.offers_reconnect());
    for state in [
        PresenceDisplay::Online,
        PresenceDisplay::Discovering,
        PresenceDisplay::Connecting,
        PresenceDisplay::Reconnecting,
    ] {
        assert!(!state.offers_reconnect());
    }
}

#[test]
fn published_online_presence_is_not_an_authenticated_ui_status() {
    let mut state = DevicesFeatureState::empty();
    state.presence = PresenceDisplay::Online;
    assert_eq!(
        state.verified_session_display(),
        VerifiedSessionDisplay::Unverified
    );
    assert_eq!(
        state.presence_control_action(),
        PresenceControlAction::Reconnect
    );
    assert!(!state.has_authenticated_session());

    let peer = DeviceId::from_bytes([0x43; 32]);
    state.current = Some(DevicePresentation::from_fields(StatusFields {
        owner_id: None,
        local_device_id: None,
        peer_device_id: Some(peer),
        trust_state: TrustState::Trusted,
        connectivity: ConnectivityState::Connected,
        session_state: SessionState::Authenticating,
        protocol_version: None,
        network_class: NetworkClass::Local,
        security_class: TransportSecurityClass::AuthenticatedConfidentialChannel,
        metered: None,
        capability_count: 0,
    }));
    assert_eq!(
        state.verified_session_display(),
        VerifiedSessionDisplay::Unverified
    );
    assert_eq!(
        state.presence_control_action(),
        PresenceControlAction::Reconnect
    );

    state.current.as_mut().unwrap().session = SessionDisplay::Active;
    assert_eq!(
        state.verified_session_display(),
        VerifiedSessionDisplay::Authenticated
    );
    assert_eq!(
        state.presence_control_action(),
        PresenceControlAction::Disconnect
    );
}

#[test]
fn revoked_peer_never_offers_reconnect_or_authenticated_label() {
    let mut state = DevicesFeatureState::empty();
    state.current = Some(DevicePresentation::from_fields(StatusFields {
        owner_id: None,
        local_device_id: None,
        peer_device_id: Some(DeviceId::from_bytes([0x44; 32])),
        trust_state: TrustState::Revoked,
        connectivity: ConnectivityState::Disconnected,
        session_state: SessionState::Revoked,
        protocol_version: None,
        network_class: NetworkClass::Local,
        security_class: TransportSecurityClass::AuthenticatedConfidentialChannel,
        metered: None,
        capability_count: 0,
    }));
    for phase in [
        PresenceDisplay::Online,
        PresenceDisplay::Failed,
        PresenceDisplay::Paused,
    ] {
        state.presence = phase;
        assert_eq!(
            state.verified_session_display(),
            VerifiedSessionDisplay::Revoked
        );
        assert_eq!(
            state.presence_control_action(),
            PresenceControlAction::Unavailable
        );
    }
}

#[test]
fn native_phase_has_a_specific_recovery_message() {
    let mut state = DevicesFeatureState::empty();
    for (presence, expected) in [
        (
            PresenceDisplay::Unavailable,
            VerifiedSessionDisplay::Unavailable,
        ),
        (
            PresenceDisplay::Discovering,
            VerifiedSessionDisplay::Discovering,
        ),
        (
            PresenceDisplay::Connecting,
            VerifiedSessionDisplay::Connecting,
        ),
        (
            PresenceDisplay::Reconnecting,
            VerifiedSessionDisplay::Reconnecting,
        ),
        (PresenceDisplay::Paused, VerifiedSessionDisplay::Paused),
        (PresenceDisplay::Failed, VerifiedSessionDisplay::Failed),
    ] {
        state.presence = presence;
        let shown = state.verified_session_display();
        assert_eq!(shown, expected);
        assert!(!shown.label().is_empty());
        assert!(!shown.detail().is_empty());
    }
    assert_eq!(
        state.presence_control_action(),
        PresenceControlAction::Reconnect
    );
}
