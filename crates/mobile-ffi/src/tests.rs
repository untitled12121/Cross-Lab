use std::{sync::Arc, thread, time::Duration};

use crosslab_core::{SessionState, TransportSecurityClass};
use crosslab_identity::{DeviceId, OwnerId};
use crosslab_policy::{NetworkClass, SessionId, TrustState};
use crosslab_protocol::ProtocolVersion;
use crosslab_runtime::ConnectivityState;

use crate::{
    MobileConnectivityState, MobileLifecycleState, MobileNetworkClass, MobileRuntime,
    MobileRuntimeError, MobileRuntimeSnapshot, MobileSessionState, MobileTransportSecurity,
    MobileTrustState, dto::RuntimeStatusFields,
};

#[test]
fn start_stop_lifecycle_is_explicit_and_clears_state() {
    let runtime = MobileRuntime::with_test_event_capacity(4);

    let initial = runtime.snapshot().expect("initial snapshot");
    assert_eq!(initial.lifecycle, MobileLifecycleState::Stopped);
    assert_eq!(initial.connectivity, MobileConnectivityState::Disconnected);
    assert_eq!(initial.trust, MobileTrustState::Unavailable);

    runtime.start().expect("start succeeds");
    let running = runtime.snapshot().expect("running snapshot");
    assert_eq!(running.lifecycle, MobileLifecycleState::Running);
    assert_eq!(running.revision, 1);
    assert!(running.peer_device_id.is_none());
    assert!(running.session_id.is_none());

    runtime.stop().expect("stop succeeds");
    let stopped = runtime.snapshot().expect("stopped snapshot");
    assert_eq!(stopped.lifecycle, MobileLifecycleState::Stopped);
    assert_eq!(stopped.revision, 2);
    assert_eq!(stopped.session, MobileSessionState::Unavailable);
    assert!(stopped.local_device_id.is_none());
    assert!(stopped.peer_device_id.is_none());
    assert!(stopped.session_id.is_none());
}

#[test]
fn duplicate_start_and_stop_without_start_are_typed() {
    let runtime = MobileRuntime::with_test_event_capacity(2);

    assert_eq!(runtime.stop(), Err(MobileRuntimeError::NotStarted));
    runtime.start().expect("first start succeeds");
    assert_eq!(runtime.start(), Err(MobileRuntimeError::AlreadyStarted));
}

#[test]
fn runtime_status_maps_to_owned_redacted_mobile_state() {
    let snapshot = MobileRuntimeSnapshot::from_test_fields(
        MobileLifecycleState::Running,
        7,
        RuntimeStatusFields {
            owner_id: Some(OwnerId::from_bytes([0xaa; 32])),
            local_device_id: Some(DeviceId::from_bytes([0xab; 32])),
            peer_device_id: Some(DeviceId::from_bytes([0xcd; 32])),
            session_id: Some(SessionId::from_bytes([0xef; 32])),
            trust_state: TrustState::Trusted,
            connectivity: ConnectivityState::Connected,
            session_state: SessionState::Active,
            protocol_version: Some(ProtocolVersion::new(1, 4)),
            network_class: NetworkClass::Local,
            security_class: TransportSecurityClass::AuthenticatedConfidentialChannel,
            metered: Some(false),
            capability_count: 2,
        },
    );

    let expected_owner = "aa".repeat(32);
    let expected_local = "ab".repeat(32);
    let expected_peer = "cd".repeat(32);
    let expected_session = "ef".repeat(32);
    assert_eq!(
        snapshot.owner_id.as_deref(),
        Some(expected_owner.as_str())
    );
    assert_eq!(
        snapshot.local_device_id.as_deref(),
        Some(expected_local.as_str())
    );
    assert_eq!(
        snapshot.peer_device_id.as_deref(),
        Some(expected_peer.as_str())
    );
    assert_eq!(
        snapshot.session_id.as_deref(),
        Some(expected_session.as_str())
    );
    assert_eq!(snapshot.trust, MobileTrustState::Trusted);
    assert_eq!(snapshot.connectivity, MobileConnectivityState::Connected);
    assert_eq!(snapshot.session, MobileSessionState::Active);
    let protocol = snapshot.protocol.expect("protocol");
    assert_eq!(protocol.major, 1);
    assert_eq!(protocol.minor, 4);
    assert_eq!(snapshot.network, MobileNetworkClass::Local);
    assert_eq!(
        snapshot.transport_security,
        MobileTransportSecurity::Authenticated
    );
    assert_eq!(snapshot.metered, Some(false));
    assert_eq!(snapshot.capability_count, 2);

    let debug = format!("{snapshot:?}").to_ascii_lowercase();
    for forbidden in [
        "channel_binding",
        "credential",
        "private_key",
        "signing_key",
        "payload",
        "quinn",
    ] {
        assert!(!debug.contains(forbidden), "snapshot leaked {forbidden}");
    }
}

#[test]
fn inactive_session_never_exports_a_stale_session_id() {
    let snapshot = MobileRuntimeSnapshot::from_test_fields(
        MobileLifecycleState::Running,
        3,
        RuntimeStatusFields {
            owner_id: Some(OwnerId::from_bytes([0; 32])),
            local_device_id: Some(DeviceId::from_bytes([1; 32])),
            peer_device_id: Some(DeviceId::from_bytes([2; 32])),
            session_id: Some(SessionId::from_bytes([3; 32])),
            trust_state: TrustState::Revoked,
            connectivity: ConnectivityState::Disconnected,
            session_state: SessionState::Revoked,
            protocol_version: None,
            network_class: NetworkClass::Remote,
            security_class: TransportSecurityClass::AuthenticatedConfidentialChannel,
            metered: None,
            capability_count: 0,
        },
    );

    assert_eq!(snapshot.trust, MobileTrustState::Revoked);
    assert_eq!(snapshot.connectivity, MobileConnectivityState::Disconnected);
    assert_eq!(snapshot.session, MobileSessionState::Revoked);
    assert_eq!(snapshot.network, MobileNetworkClass::Remote);
    assert!(snapshot.session_id.is_none());
}

#[test]
fn slow_consumer_sees_a_bounded_latest_snapshot_queue() {
    let runtime = MobileRuntime::with_test_event_capacity(2);
    runtime.start().expect("start succeeds");

    runtime
        .publish_test_snapshot(MobileRuntimeSnapshot::disconnected(
            MobileLifecycleState::Running,
            2,
        ))
        .expect("publish revision 2");
    runtime
        .publish_test_snapshot(MobileRuntimeSnapshot::disconnected(
            MobileLifecycleState::Running,
            3,
        ))
        .expect("publish revision 3");

    assert_eq!(
        runtime
            .poll_event()
            .expect("poll")
            .expect("revision 2")
            .snapshot
            .revision,
        2
    );
    assert_eq!(
        runtime
            .poll_event()
            .expect("poll")
            .expect("revision 3")
            .snapshot
            .revision,
        3
    );
    assert!(runtime.poll_event().expect("poll").is_none());
}

#[test]
fn ffi_errors_do_not_embed_sensitive_context() {
    for error in [
        MobileRuntimeError::AlreadyStarted,
        MobileRuntimeError::NotStarted,
        MobileRuntimeError::StateUnavailable,
    ] {
        let rendered = format!("{error:?} {error}").to_ascii_lowercase();
        for forbidden in [
            "credential",
            "private",
            "secret",
            "channel",
            "payload",
            "endpoint",
        ] {
            assert!(!rendered.contains(forbidden), "error leaked {forbidden}");
        }
    }
}

#[test]
fn blocking_event_wait_wakes_without_polling() {
    let runtime = Arc::new(MobileRuntime::with_test_event_capacity(2));
    runtime.start().expect("start succeeds");
    runtime.poll_event().expect("drain start event");

    let waiter = Arc::clone(&runtime);
    let thread = thread::spawn(move || {
        waiter
            .wait_event(1_000)
            .expect("wait succeeds")
            .expect("published event")
            .snapshot
            .revision
    });

    thread::sleep(Duration::from_millis(10));
    runtime
        .publish_test_snapshot(MobileRuntimeSnapshot::disconnected(
            MobileLifecycleState::Running,
            2,
        ))
        .expect("publish succeeds");

    assert_eq!(thread.join().expect("waiter joins"), 2);
}

#[test]
fn blocking_event_wait_times_out_cleanly() {
    let runtime = MobileRuntime::with_test_event_capacity(2);

    assert!(runtime.wait_event(1).expect("wait succeeds").is_none());
}

#[test]
fn network_loss_clears_session_facing_state_while_running() {
    let runtime = MobileRuntime::with_test_event_capacity(4);
    runtime.start().expect("start succeeds");
    runtime.poll_event().expect("drain start event");

    runtime
        .publish_test_snapshot(MobileRuntimeSnapshot {
            revision: 2,
            lifecycle: MobileLifecycleState::Running,
            owner_id: Some("owner".into()),
            local_device_id: Some("local".into()),
            peer_device_id: Some("peer".into()),
            session_id: Some("session".into()),
            trust: MobileTrustState::Trusted,
            connectivity: MobileConnectivityState::Connected,
            session: MobileSessionState::Active,
            protocol: None,
            network: MobileNetworkClass::Local,
            transport_security: MobileTransportSecurity::Authenticated,
            metered: Some(false),
            capability_count: 3,
        })
        .expect("publish connected state");
    runtime.poll_event().expect("drain connected event");

    runtime.network_lost().expect("network loss succeeds");
    let snapshot = runtime.snapshot().expect("snapshot");
    assert_eq!(snapshot.lifecycle, MobileLifecycleState::Running);
    assert_eq!(snapshot.connectivity, MobileConnectivityState::Disconnected);
    assert_eq!(snapshot.trust, MobileTrustState::Unavailable);
    assert_eq!(snapshot.session, MobileSessionState::Unavailable);
    assert!(snapshot.peer_device_id.is_none());
    assert!(snapshot.session_id.is_none());
}

#[test]
fn network_commands_require_running_lifecycle() {
    let runtime = MobileRuntime::with_test_event_capacity(2);

    assert_eq!(runtime.network_lost(), Err(MobileRuntimeError::NotStarted));
    assert_eq!(
        runtime.network_available(),
        Err(MobileRuntimeError::NotStarted)
    );
    assert_eq!(runtime.disconnect_peer(), Err(MobileRuntimeError::NotStarted));
    assert_eq!(runtime.reconnect_peer(), Err(MobileRuntimeError::NotStarted));

    runtime.start().expect("start succeeds");
    runtime
        .network_available()
        .expect("network available succeeds while running");

    #[cfg(not(feature = "development-provisioning"))]
    {
        assert_eq!(
            runtime.disconnect_peer(),
            Err(MobileRuntimeError::DevelopmentUnavailable)
        );
        assert_eq!(
            runtime.reconnect_peer(),
            Err(MobileRuntimeError::DevelopmentUnavailable)
        );
    }
}
