use std::{num::NonZeroUsize, time::Duration};

use crosslab_core::{
    ControlReceiveError, SessionActivation, SessionError, SessionHandshakeSide, SessionState,
    StreamAcceptError, StreamAdmissionError, StreamReceiveError, StreamSendError,
    TransportConnection, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_policy::{
    AuthorizationContext, AuthorizedOperation, CapabilityId, CapabilityVersion,
    CapabilityVersionRange, LocalCapability, NetworkClass, OperationId, OperationName, PolicyRule,
    PolicyState, RuleEffect, RuleId, TransitionId, TrustRecord, TrustState, TrustTransition,
    UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlRequest, ControlResponseResult,
    DataStreamOpen, Event, EventId, EventType, RequestId, RetryClass, StreamDirection, StreamId,
};
use crosslab_sim::{
    node::{NodeError, NodeEvent, SimNode},
    stream::{SimStreamError, SimStreamRuntime},
};
use tokio::time::timeout;

use super::*;

const WAIT: Duration = Duration::from_secs(2);
const STATE_CAPACITY: usize = 16;
const STREAM_CAPACITY: usize = 8;

struct StreamAuthority {
    capability: CapabilityId,
    version: CapabilityVersion,
    operation_name: OperationName,
    operation: AuthorizedOperation,
    trust_revision: u64,
    policy_revision: u64,
    session_id: crosslab_policy::SessionId,
}

#[tokio::test]
async fn m8_authenticated_quinn_carries_capabilities_control_response_and_event() {
    let fixture = AuthFixture::new();
    let pair = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("authenticated Quinn pair should be available");
    let AuthenticatedLoopbackSessionPair {
        _client_endpoint,
        _server_endpoint,
        client_transport,
        server_transport,
        client_session,
        server_session,
        ..
    } = pair;

    assert_eq!(
        client_transport.security_class(),
        TransportSecurityClass::AuthenticatedConfidentialChannel
    );
    assert_eq!(
        client_transport.channel_binding(),
        server_transport.channel_binding()
    );

    let local_capabilities = clipboard_local_capabilities();
    let mut client = SimNode::new(
        client_session,
        &client_transport,
        PolicyState::new(),
        local_capabilities.clone(),
        nonzero(STATE_CAPACITY),
    )
    .unwrap();
    let mut server = SimNode::new(
        server_session,
        &server_transport,
        allow_clipboard_policy(fixture.initiator_credential.device_id()),
        local_capabilities,
        nonzero(STATE_CAPACITY),
    )
    .unwrap();

    client
        .send_capability_advertisement(clipboard_advertisement())
        .unwrap();
    assert!(matches!(
        eventually_node_event(&mut server).await,
        NodeEvent::CapabilitiesUpdated
    ));
    server
        .send_capability_advertisement(clipboard_advertisement())
        .unwrap();
    assert!(matches!(
        eventually_node_event(&mut client).await,
        NodeEvent::CapabilitiesUpdated
    ));

    let request_id = RequestId::from_bytes([0x90; 16]);
    client
        .send_request(clipboard_request(request_id, b"network-control"))
        .unwrap();
    let NodeEvent::RequestDispatched(request) = eventually_node_event(&mut server).await else {
        panic!("expected authorized control request");
    };
    assert_eq!(request.request_id(), request_id);
    assert_eq!(request.body(), b"network-control");

    server
        .send_response(request_id, ControlResponseResult::Success(b"ok".to_vec()))
        .unwrap();
    let NodeEvent::Response(response) = eventually_node_event(&mut client).await else {
        panic!("expected correlated control response");
    };
    assert_eq!(response.request_id(), request_id);

    let event_id = EventId::from_bytes([0x91; 16]);
    client
        .send_event(clipboard_event(event_id, b"changed"))
        .unwrap();
    let NodeEvent::Event(event) = eventually_node_event(&mut server).await else {
        panic!("expected negotiated capability event");
    };
    assert_eq!(event.event_id(), event_id);
    assert_eq!(event.body(), b"changed");

    client.shutdown();
    server.shutdown();
    drop((client, server));
    shutdown_pair(&client_transport, &server_transport).await;
}

#[tokio::test]
async fn m8_operation_bound_stream_flows_and_unknown_operation_is_rejected() {
    let fixture = AuthFixture::new();
    let pair = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("authenticated Quinn pair should be available");
    let AuthenticatedLoopbackSessionPair {
        _client_endpoint,
        _server_endpoint,
        client_transport,
        server_transport,
        mut client_session,
        mut server_session,
        ..
    } = pair;
    let authority = prepare_stream_authority(&fixture, &mut client_session, &mut server_session);
    let mut sender =
        SimStreamRuntime::new(client_session, &client_transport, nonzero(STREAM_CAPACITY)).unwrap();
    let mut receiver =
        SimStreamRuntime::new(server_session, &server_transport, nonzero(STREAM_CAPACITY)).unwrap();
    receiver
        .register_operation(authority.operation.clone())
        .unwrap();

    let open = stream_open(
        &authority,
        authority.operation.id(),
        StreamId::from_bytes([0x92; 16]),
    );
    let mut send = sender.open_uni(&open).unwrap();
    let stream_id = eventually_accept(
        &mut receiver,
        15,
        authority.trust_revision,
        authority.policy_revision,
    )
    .await
    .expect("authorized stream should be admitted");
    assert_eq!(stream_id, open.stream_id());

    send.try_send_chunk(b"payload".to_vec()).unwrap();
    assert_eq!(
        eventually_receive(&mut receiver, stream_id).await,
        b"payload"
    );
    send.finish();
    eventually_finished(&mut receiver, stream_id).await;

    let unknown = stream_open(
        &authority,
        OperationId::from_bytes([0x93; 32]),
        StreamId::from_bytes([0x94; 16]),
    );
    let mut unknown_send = sender.open_uni(&unknown).unwrap();
    assert!(matches!(
        eventually_accept(
            &mut receiver,
            15,
            authority.trust_revision,
            authority.policy_revision,
        )
        .await,
        Err(SimStreamError::Admission(
            StreamAdmissionError::OperationNotFound
        ))
    ));
    eventually_sender_closed(unknown_send.as_mut()).await;

    sender.shutdown();
    receiver.shutdown();
    drop((sender, receiver));
    shutdown_pair(&client_transport, &server_transport).await;
}

#[tokio::test]
async fn m8_reconnect_reauthenticates_with_fresh_authority() {
    let fixture = AuthFixture::new();
    let first = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("first authenticated connection should succeed");
    let AuthenticatedLoopbackSessionPair {
        _client_endpoint,
        _server_endpoint,
        client_transport,
        server_transport,
        mut client_session,
        server_session: _,
        ..
    } = first;
    let first_binding = client_transport.channel_binding().bytes().to_vec();
    let first_session_id = client_session.context().unwrap().session_id();
    let local_capabilities = clipboard_local_capabilities();
    client_session
        .negotiate_capabilities(&local_capabilities, &clipboard_advertisement())
        .unwrap();
    let mut client = SimNode::new(
        client_session,
        &client_transport,
        PolicyState::new(),
        local_capabilities,
        nonzero(STATE_CAPACITY),
    )
    .unwrap();
    client
        .send_request(clipboard_request(
            RequestId::from_bytes([0x95; 16]),
            b"pending",
        ))
        .unwrap();
    assert_eq!(client.pending_request_count(), 1);

    server_transport.close();
    eventually_transport_closed(&client_transport).await;
    eventually_node_transport_loss(&mut client).await;
    assert_eq!(client.session().state(), SessionState::Closed);
    assert_eq!(client.pending_request_count(), 0);
    drop(client);
    shutdown_pair(&client_transport, &server_transport).await;

    let reconnect = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("fresh authentication should succeed");
    assert_ne!(
        first_binding,
        reconnect.client_transport.channel_binding().bytes()
    );
    assert_ne!(
        first_session_id,
        reconnect.client_session.context().unwrap().session_id()
    );
    let mut fresh_node = SimNode::new(
        reconnect.client_session,
        &reconnect.client_transport,
        PolicyState::new(),
        clipboard_local_capabilities(),
        nonzero(STATE_CAPACITY),
    )
    .unwrap();
    assert_eq!(fresh_node.pending_request_count(), 0);
    assert_eq!(fresh_node.next_send_sequence(), Some(0));
    assert_eq!(fresh_node.expected_receive_sequence(), Some(0));
    fresh_node.shutdown();
    drop(fresh_node);
    shutdown_pair(&reconnect.client_transport, &reconnect.server_transport).await;
}

#[tokio::test]
async fn m8_old_operation_state_is_rejected_after_reconnect() {
    let fixture = AuthFixture::new();
    let first = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("first authenticated connection should succeed");
    let old_session_id = first.client_session.context().unwrap().session_id();
    first.shutdown().await;

    let reconnect = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("fresh authentication should succeed");
    let AuthenticatedLoopbackSessionPair {
        _client_endpoint,
        _server_endpoint,
        client_transport,
        server_transport,
        client_session,
        server_session,
        ..
    } = reconnect;
    let mut sender =
        SimStreamRuntime::new(client_session, &client_transport, nonzero(STREAM_CAPACITY)).unwrap();
    let mut receiver =
        SimStreamRuntime::new(server_session, &server_transport, nonzero(STREAM_CAPACITY)).unwrap();
    let open = DataStreamOpen::new(
        old_session_id,
        StreamId::from_bytes([0x96; 16]),
        OperationId::from_bytes([0x97; 32]),
        files_capability(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("send").unwrap(),
        StreamDirection::SourceToDestination,
        0,
    );
    let mut send = sender.open_uni(&open).unwrap();
    assert!(matches!(
        eventually_accept(&mut receiver, 15, 0, 0).await,
        Err(SimStreamError::Admission(
            StreamAdmissionError::InvalidSession
        ))
    ));
    eventually_sender_closed(send.as_mut()).await;

    sender.shutdown();
    receiver.shutdown();
    drop((sender, receiver));
    shutdown_pair(&client_transport, &server_transport).await;
}

#[tokio::test]
async fn m8_active_revocation_cancels_stream_authority_and_fresh_reconnect_is_denied() {
    let fixture = AuthFixture::new();
    let pair = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("authenticated Quinn pair should be available");
    let AuthenticatedLoopbackSessionPair {
        _client_endpoint,
        _server_endpoint,
        client_transport,
        server_transport,
        mut client_session,
        mut server_session,
        ..
    } = pair;
    let authority = prepare_stream_authority(&fixture, &mut client_session, &mut server_session);
    let mut sender =
        SimStreamRuntime::new(client_session, &client_transport, nonzero(STREAM_CAPACITY)).unwrap();
    let mut receiver =
        SimStreamRuntime::new(server_session, &server_transport, nonzero(STREAM_CAPACITY)).unwrap();
    receiver
        .register_operation(authority.operation.clone())
        .unwrap();
    let open = stream_open(
        &authority,
        authority.operation.id(),
        StreamId::from_bytes([0x98; 16]),
    );
    let mut send = sender.open_uni(&open).unwrap();
    let stream_id = eventually_accept(
        &mut receiver,
        15,
        authority.trust_revision,
        authority.policy_revision,
    )
    .await
    .unwrap();

    let revoked = revoked_responder(&fixture);
    sender.apply_peer_revocation(&revoked).unwrap();
    assert_eq!(sender.session().state(), SessionState::Closed);
    eventually_transport_closed(&server_transport).await;
    assert!(matches!(
        eventually_receive_result(&mut receiver, stream_id).await,
        Err(SimStreamError::Receive(StreamReceiveError::Cancelled))
    ));
    assert_eq!(receiver.session().state(), SessionState::Closed);
    assert_eq!(
        send.try_send_chunk(b"after-revoke".to_vec()),
        Err(StreamSendError::Closed(b"after-revoke".to_vec()))
    );
    drop((sender, receiver));
    shutdown_pair(&client_transport, &server_transport).await;

    let fresh = bootstrap_loopback_session_pair(&fixture).await;
    let initiator_nonce = nonce_for_binding(&fresh.client_binding, b"initiator");
    let responder_nonce = nonce_for_binding(&fresh.client_binding, b"responder");
    let ranges_a = AuthFixture::initiator_ranges();
    let ranges_b = AuthFixture::responder_ranges();
    let features_a = AuthFixture::initiator_features();
    let features_b = AuthFixture::responder_features();
    let protocol = negotiate_protocol_version(&ranges_a, &ranges_b).unwrap();
    let features = negotiate_features(&features_a, &features_b).unwrap();
    let transcript = SessionAuthTranscriptV1::new(
        fixture.owner_id,
        &fixture.initiator_credential,
        initiator_nonce,
        &fixture.responder_credential,
        responder_nonce,
        protocol,
        &features,
        fresh.client_binding.profile_id().as_bytes(),
        fresh.client_binding.bytes(),
    )
    .unwrap();
    let initiator_proof = transcript
        .create_proof(CoreSessionAuthRole::Initiator, &fixture.initiator_key)
        .unwrap();
    let responder_proof = transcript
        .create_proof(CoreSessionAuthRole::Responder, &fixture.responder_key)
        .unwrap();
    let initiator_side = SessionHandshakeSide::new(
        &fixture.initiator_credential,
        &fixture.delegation,
        &ranges_a,
        &features_a,
    );
    let responder_side = SessionHandshakeSide::new(
        &fixture.responder_credential,
        &fixture.delegation,
        &ranges_b,
        &features_b,
    );
    let mut reconnect_session = LogicalSession::new();
    assert_eq!(
        reconnect_session.authenticate(SessionActivation::new(
            &fixture.root,
            initiator_side,
            responder_side,
            CoreSessionAuthRole::Initiator,
            &revoked,
            initiator_nonce,
            responder_nonce,
            &fresh.client_binding,
            TransportSecurityClass::AuthenticatedConfidentialChannel,
            &initiator_proof,
            &responder_proof,
        )),
        Err(SessionError::PeerNotTrusted)
    );
    assert_eq!(reconnect_session.state(), SessionState::Closed);
    fresh.shutdown().await;
}

#[tokio::test]
async fn m8_saturation_cancellation_and_shutdown_remain_bounded() {
    let fixture = AuthFixture::new();
    let pair = authenticate_loopback_session_pair(&fixture, AuthAttempt::Normal)
        .await
        .expect("authenticated Quinn pair should be available");
    let capacity = QuicTransportConfig::default().outgoing_stream_capacity();
    let mut streams = Vec::with_capacity(capacity);
    for index in 0..capacity {
        streams.push(
            pair.client_transport
                .try_open_uni_stream(vec![u8::try_from(index).unwrap()])
                .unwrap(),
        );
    }
    let opening = b"owned-on-full".to_vec();
    let error = match pair.client_transport.try_open_uni_stream(opening.clone()) {
        Err(error) => error,
        Ok(_) => panic!("outgoing stream slots must remain bounded"),
    };
    assert_eq!(error, crosslab_core::StreamOpenError::Full(opening));

    for stream in &mut streams {
        stream.cancel();
    }
    let mut active = pair
        .client_transport
        .try_open_uni_stream(vec![0x99])
        .unwrap();
    timeout(WAIT, pair.client_transport.shutdown())
        .await
        .expect("client shutdown should join active stream tasks");
    assert_eq!(
        active.try_send_chunk(b"after-shutdown".to_vec()),
        Err(StreamSendError::Closed(b"after-shutdown".to_vec()))
    );
    timeout(WAIT, pair.server_transport.shutdown())
        .await
        .expect("server shutdown should join active stream tasks");
}

fn clipboard_capability() -> CapabilityId {
    CapabilityId::parse("clipboard.write").unwrap()
}

fn files_capability() -> CapabilityId {
    CapabilityId::parse("files.transfer").unwrap()
}

fn clipboard_local_capabilities() -> Vec<LocalCapability> {
    vec![LocalCapability::new(
        clipboard_capability(),
        CapabilityVersionRange::new(1, 0, 0).unwrap(),
        true,
    )]
}

fn clipboard_advertisement() -> CapabilityAdvertisement {
    CapabilityAdvertisement::new(vec![
        CapabilityAdvertisementEntry::new(
            clipboard_capability(),
            CapabilityVersion::new(1, 0),
            CapabilityVersion::new(1, 0),
            true,
        )
        .unwrap(),
    ])
    .unwrap()
}

fn allow_clipboard_policy(source: crosslab_identity::DeviceId) -> PolicyState {
    let mut policy = PolicyState::new();
    policy
        .insert(PolicyRule::new(
            RuleId::from_bytes([0xa0; 32]),
            source,
            clipboard_capability(),
            OperationName::parse("set").unwrap(),
            RuleEffect::Allow,
        ))
        .unwrap();
    policy
}

fn clipboard_request(request_id: RequestId, body: &[u8]) -> ControlRequest {
    ControlRequest::new(
        request_id,
        clipboard_capability(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("set").unwrap(),
        RetryClass::NonRetryable,
        body.to_vec(),
    )
}

fn clipboard_event(event_id: EventId, body: &[u8]) -> Event {
    Event::capability(
        event_id,
        clipboard_capability(),
        EventType::parse("clipboard.changed").unwrap(),
        body.to_vec(),
    )
    .unwrap()
}

fn prepare_stream_authority(
    fixture: &AuthFixture,
    client_session: &mut LogicalSession,
    server_session: &mut LogicalSession,
) -> StreamAuthority {
    let capability = files_capability();
    let version = CapabilityVersion::new(1, 0);
    let operation_name = OperationName::parse("send").unwrap();
    let local_capability = LocalCapability::new(
        capability.clone(),
        CapabilityVersionRange::new(1, 0, 0).unwrap(),
        true,
    );
    let advertisement = CapabilityAdvertisement::new(vec![
        CapabilityAdvertisementEntry::new(capability.clone(), version, version, true).unwrap(),
    ])
    .unwrap();
    client_session
        .negotiate_capabilities(std::slice::from_ref(&local_capability), &advertisement)
        .unwrap();
    server_session
        .negotiate_capabilities(std::slice::from_ref(&local_capability), &advertisement)
        .unwrap();

    let session_id = server_session.context().unwrap().session_id();
    let trust_revision = fixture.initiator_trust.trust_revision();
    let mut policy = PolicyState::new();
    policy
        .insert(PolicyRule::new(
            RuleId::from_bytes([0xa1; 32]),
            fixture.initiator_credential.device_id(),
            capability.clone(),
            operation_name.clone(),
            RuleEffect::Allow,
        ))
        .unwrap();
    let policy_revision = policy.revision();
    let context = AuthorizationContext::new(
        fixture.initiator_credential.device_id(),
        fixture.responder_credential.device_id(),
        session_id,
        capability.clone(),
        version,
        operation_name.clone(),
        TrustState::Trusted,
        trust_revision,
        local_capability,
        NetworkClass::Local,
    );
    let grant = policy.evaluate(&context).into_grant().unwrap();
    let operation = AuthorizedOperation::issue(grant, 10, 20, UsePolicy::SingleStream).unwrap();

    StreamAuthority {
        capability,
        version,
        operation_name,
        operation,
        trust_revision,
        policy_revision,
        session_id,
    }
}

fn stream_open(
    authority: &StreamAuthority,
    operation_id: OperationId,
    stream_id: StreamId,
) -> DataStreamOpen {
    DataStreamOpen::new(
        authority.session_id,
        stream_id,
        operation_id,
        authority.capability.clone(),
        authority.version,
        authority.operation_name.clone(),
        StreamDirection::SourceToDestination,
        0,
    )
}

fn revoked_responder(fixture: &AuthFixture) -> TrustRecord {
    let root_key = SigningKey::from_secret_bytes([0x41; 32]);
    let transition = TrustTransition::issue_root_revocation(
        &fixture.responder_trust,
        TransitionId::from_bytes([0xa2; 32]),
        &fixture.root,
        &root_key,
    )
    .unwrap();
    let mut revoked = fixture.responder_trust;
    transition.apply_root(&mut revoked, &fixture.root).unwrap();
    revoked
}

async fn eventually_node_event(node: &mut SimNode<'_>) -> NodeEvent {
    timeout(WAIT, async {
        loop {
            match node.receive_one() {
                Ok(event) => return event,
                Err(NodeError::Receive(ControlReceiveError::Empty)) => {
                    tokio::task::yield_now().await
                }
                Err(error) => panic!("node failed before expected event: {error:?}"),
            }
        }
    })
    .await
    .expect("node event was not delivered before timeout")
}

async fn eventually_node_transport_loss(node: &mut SimNode<'_>) {
    timeout(WAIT, async {
        loop {
            match node.receive_one() {
                Err(NodeError::Receive(ControlReceiveError::Closed)) => return,
                Err(NodeError::Receive(ControlReceiveError::Empty)) => {
                    tokio::task::yield_now().await
                }
                Ok(_) => tokio::task::yield_now().await,
                Err(error) => panic!("unexpected node error while waiting for close: {error:?}"),
            }
        }
    })
    .await
    .expect("node did not observe transport loss before timeout");
}

async fn eventually_accept(
    runtime: &mut SimStreamRuntime<'_>,
    now: u64,
    trust_revision: u64,
    policy_revision: u64,
) -> Result<StreamId, SimStreamError> {
    timeout(WAIT, async {
        loop {
            match runtime.accept_one(now, trust_revision, policy_revision) {
                Err(SimStreamError::Accept(StreamAcceptError::Empty)) => {
                    tokio::task::yield_now().await
                }
                result => return result,
            }
        }
    })
    .await
    .expect("stream accept did not resolve before timeout")
}

async fn eventually_receive(runtime: &mut SimStreamRuntime<'_>, stream_id: StreamId) -> Vec<u8> {
    timeout(WAIT, async {
        loop {
            match runtime.try_receive_chunk(stream_id) {
                Ok(chunk) => return chunk,
                Err(SimStreamError::Receive(StreamReceiveError::Empty)) => {
                    tokio::task::yield_now().await
                }
                Err(error) => panic!("stream failed before payload arrived: {error:?}"),
            }
        }
    })
    .await
    .expect("stream payload was not delivered before timeout")
}

async fn eventually_receive_result(
    runtime: &mut SimStreamRuntime<'_>,
    stream_id: StreamId,
) -> Result<Vec<u8>, SimStreamError> {
    timeout(WAIT, async {
        loop {
            match runtime.try_receive_chunk(stream_id) {
                Err(SimStreamError::Receive(StreamReceiveError::Empty)) => {
                    tokio::task::yield_now().await
                }
                result => return result,
            }
        }
    })
    .await
    .expect("stream terminal result did not arrive before timeout")
}

async fn eventually_finished(runtime: &mut SimStreamRuntime<'_>, stream_id: StreamId) {
    assert!(matches!(
        eventually_receive_result(runtime, stream_id).await,
        Err(SimStreamError::Receive(StreamReceiveError::Finished))
    ));
}

async fn eventually_sender_closed(stream: &mut dyn crosslab_core::TransportSendStream) {
    timeout(WAIT, async {
        loop {
            let chunk = b"probe".to_vec();
            match stream.try_send_chunk(chunk.clone()) {
                Err(StreamSendError::Closed(returned)) => {
                    assert_eq!(returned, chunk);
                    return;
                }
                Ok(()) | Err(StreamSendError::Full(_)) => tokio::task::yield_now().await,
                Err(error) => panic!("unexpected sender state: {error:?}"),
            }
        }
    })
    .await
    .expect("sender did not observe peer cancellation before timeout");
}

async fn eventually_transport_closed(connection: &QuicTransportConnection) {
    timeout(WAIT, async {
        while !connection.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("transport did not become terminal before timeout");
}

async fn shutdown_pair(client: &QuicTransportConnection, server: &QuicTransportConnection) {
    timeout(WAIT, async {
        tokio::join!(client.shutdown(), server.shutdown());
    })
    .await
    .expect("transport pair did not shut down before timeout");
}

fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
