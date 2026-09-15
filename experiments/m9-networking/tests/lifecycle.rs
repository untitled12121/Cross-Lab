use std::time::Duration;

use crosslab_core::{
    ControlDispatchError, SessionAuthError, SessionError, SessionState, StreamAdmissionError,
    StreamOpenError, StreamSendError, TransportConnection,
};
use crosslab_m9_networking::{
    candidate::runtime::CandidateConfig,
    scenarios::{
        auth::{AuthAttempt, AuthFixture, authenticate_direct_pair},
        lifecycle::{
            exercise_control_lifecycle, exercise_reconnect_lifecycle,
            exercise_revocation_lifecycle, exercise_stream_lifecycle,
        },
    },
};
use crosslab_policy::{
    AuthorizationContext, CapabilityId, CapabilityVersion, CapabilityVersionRange, Constraint,
    DecisionReason, LocalCapability, NetworkClass, OperationError, OperationName, PolicyRule,
    PolicyState, RuleEffect, RuleId, TrustState,
};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn authenticated_iroh_carries_capability_control_response_and_event() {
    let evidence = exercise_control_lifecycle(&AuthFixture::new()).await;

    assert_eq!(evidence.network_class, NetworkClass::Remote);
    assert_eq!(evidence.request_body, b"network-control");
    assert_eq!(evidence.response_body, b"ok");
    assert_eq!(evidence.event_body, b"changed");
}

#[tokio::test]
async fn operation_bound_stream_flows_and_unknown_operation_is_rejected() {
    let evidence = exercise_stream_lifecycle(&AuthFixture::new()).await;

    assert_eq!(evidence.payload, b"payload");
    assert_eq!(
        evidence.unknown_operation_error,
        StreamAdmissionError::OperationNotFound
    );
}

#[tokio::test]
async fn reconnect_reauthenticates_and_rejects_old_authority() {
    let evidence = exercise_reconnect_lifecycle(&AuthFixture::new()).await;

    assert_ne!(evidence.old_binding, evidence.new_binding);
    assert_ne!(evidence.old_session_id, evidence.new_session_id);
    assert_eq!(
        evidence.old_proof_error,
        SessionError::Auth(SessionAuthError::WrongProofTranscript)
    );
    assert_eq!(
        evidence.old_session_error,
        ControlDispatchError::InvalidSession
    );
    assert_eq!(
        evidence.old_operation_error,
        StreamAdmissionError::Operation(OperationError::BindingMismatch)
    );
    assert_eq!(evidence.pending_request_count, 0);
    assert_eq!(evidence.next_send_sequence, Some(0));
    assert_eq!(evidence.expected_receive_sequence, Some(0));
}

#[tokio::test]
async fn signed_revocation_terminates_authority_and_denies_reconnect() {
    let evidence = exercise_revocation_lifecycle(&AuthFixture::new()).await;

    assert_eq!(evidence.local_state, SessionState::Closed);
    assert_eq!(evidence.remote_state, SessionState::Closed);
    assert!(evidence.stream_cancelled);
    assert!(evidence.sender_closed);
    assert_eq!(evidence.reconnect_error, SessionError::PeerNotTrusted);
}

#[tokio::test]
async fn saturation_cancellation_and_shutdown_remain_bounded() {
    let fixture = AuthFixture::new();
    let pair = authenticate_direct_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("authenticated Iroh pair");
    let capacity = CandidateConfig::default().outgoing_stream_capacity();
    let mut streams = Vec::with_capacity(capacity);

    for index in 0..capacity {
        streams.push(
            pair.client_transport()
                .try_open_uni_stream(vec![u8::try_from(index).unwrap()])
                .expect("stream slot inside configured bound"),
        );
    }

    let opening = b"owned-on-full".to_vec();
    match pair.client_transport().try_open_uni_stream(opening.clone()) {
        Err(StreamOpenError::Full(returned)) => assert_eq!(returned, opening),
        Err(error) => panic!("unexpected saturation error: {error:?}"),
        Ok(_) => panic!("outgoing stream slots must remain bounded"),
    }

    for stream in &mut streams {
        stream.cancel();
    }

    timeout(WAIT, pair.shutdown())
        .await
        .expect("Iroh lifecycle shutdown should join owned tasks");

    for stream in &mut streams {
        let payload = b"after-shutdown".to_vec();
        assert_eq!(
            stream.try_send_chunk(payload.clone()),
            Err(StreamSendError::Closed(payload))
        );
    }
}

#[tokio::test]
async fn local_only_policy_rejects_remote_iroh_session() {
    let fixture = AuthFixture::new();
    let pair = authenticate_direct_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("authenticated Iroh pair");
    let context = pair.server_session().context().expect("active session context");
    let capability = CapabilityId::parse("files.transfer").unwrap();
    let version = CapabilityVersion::new(1, 0);
    let operation = OperationName::parse("send").unwrap();
    let local_capability = LocalCapability::new(
        capability.clone(),
        CapabilityVersionRange::new(1, 0, 0).unwrap(),
        true,
    );
    let rule = PolicyRule::new(
        RuleId::from_bytes([0xc0; 32]),
        context.peer_device_id(),
        capability.clone(),
        operation.clone(),
        RuleEffect::Allow,
    )
    .with_constraint(Constraint::LocalOnly);
    let mut policy = PolicyState::new();
    policy.insert(rule).unwrap();
    let authorization = AuthorizationContext::new(
        context.peer_device_id(),
        context.local_device_id(),
        context.session_id(),
        capability,
        version,
        operation,
        TrustState::Trusted,
        context.peer_trust_revision(),
        local_capability,
        pair.network_class(),
    );

    assert_eq!(pair.network_class(), NetworkClass::Remote);
    assert_eq!(
        policy.evaluate(&authorization).reason(),
        DecisionReason::ConstraintFailed
    );

    pair.shutdown().await;
}
