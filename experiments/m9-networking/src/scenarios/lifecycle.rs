use std::{num::NonZeroUsize, time::Duration};

use crosslab_core::{
    ControlDispatchError, ControlReceiveError, EventSubscription, LogicalSession, SessionError,
    SessionState, StreamAcceptError, StreamAdmissionError, StreamReceiveError, StreamSendError,
    TransportConnection, TransportSendStream,
};
use crosslab_policy::{
    AuthorizationContext, AuthorizedOperation, CapabilityId, CapabilityVersion,
    CapabilityVersionRange, LocalCapability, NetworkClass, OperationId, OperationName, PolicyRule,
    PolicyState, RuleEffect, RuleId, SessionId, TrustRecord, TrustState, UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlEnvelope, ControlRequest,
    ControlResponseResult, DataStreamOpen, EnvelopeBody, Event, EventId, EventType, RequestId,
    RetryClass, StreamDirection, StreamId, encode_control_envelope,
};
use crosslab_sim::{
    node::{NodeError, NodeEvent, SimNode},
    stream::{SimStreamError, SimStreamRuntime},
};
use tokio::time::timeout;

use crate::{
    candidate::{connection::IrohTransportConnection, endpoint::DirectPair},
    scenarios::auth::{
        AuthAttempt, AuthFixture, AuthenticatedIrohParts, authenticate_direct_pair,
        authenticate_direct_pair_with_peer_trust,
    },
};

const WAIT: Duration = Duration::from_secs(5);
const STATE_CAPACITY: usize = 16;
const STREAM_CAPACITY: usize = 8;

pub struct ControlLifecycleEvidence {
    pub network_class: NetworkClass,
    pub request_body: Vec<u8>,
    pub response_body: Vec<u8>,
    pub event_body: Vec<u8>,
}

pub struct StreamLifecycleEvidence {
    pub payload: Vec<u8>,
    pub unknown_operation_error: StreamAdmissionError,
}

pub struct ReconnectLifecycleEvidence {
    pub old_binding: Vec<u8>,
    pub new_binding: Vec<u8>,
    pub old_session_id: SessionId,
    pub new_session_id: SessionId,
    pub old_proof_error: SessionError,
    pub old_session_error: ControlDispatchError,
    pub old_operation_error: StreamAdmissionError,
    pub pending_request_count: usize,
    pub next_send_sequence: Option<u64>,
    pub expected_receive_sequence: Option<u64>,
}

pub struct RevocationLifecycleEvidence {
    pub local_state: SessionState,
    pub remote_state: SessionState,
    pub stream_cancelled: bool,
    pub sender_closed: bool,
    pub reconnect_error: SessionError,
}

struct StreamAuthority {
    capability: CapabilityId,
    version: CapabilityVersion,
    operation_name: OperationName,
    operation: AuthorizedOperation,
    peer_trust: TrustRecord,
    policy: PolicyState,
    session_id: SessionId,
}

struct StreamContext {
    capability: CapabilityId,
    version: CapabilityVersion,
    operation_name: OperationName,
    peer_trust: TrustRecord,
    policy: PolicyState,
    session_id: SessionId,
}

pub async fn exercise_control_lifecycle(fixture: &AuthFixture) -> ControlLifecycleEvidence {
    let pair = authenticate_direct_pair(fixture, AuthAttempt::Normal)
        .await
        .expect("authenticated Iroh pair");
    let network_class = pair.network_class();
    let AuthenticatedIrohParts {
        direct,
        client_transport,
        server_transport,
        client_session,
        server_session,
    } = pair.into_parts();

    let local_capabilities = clipboard_local_capabilities();
    let mut client = SimNode::new(
        client_session,
        &client_transport,
        PolicyState::new(),
        local_capabilities.clone(),
        NetworkClass::Remote,
        nonzero(STATE_CAPACITY),
    )
    .unwrap();
    let mut server = SimNode::new(
        server_session,
        &server_transport,
        allow_clipboard_policy(fixture.initiator_credential().device_id()),
        local_capabilities,
        NetworkClass::Remote,
        nonzero(STATE_CAPACITY),
    )
    .unwrap();

    client
        .send_capability_advertisement(clipboard_advertisement())
        .unwrap();
    assert!(matches!(
        eventually_node_event(&mut server, &fixture.initiator_trust()).await,
        NodeEvent::CapabilitiesUpdated
    ));
    server
        .send_capability_advertisement(clipboard_advertisement())
        .unwrap();
    assert!(matches!(
        eventually_node_event(&mut client, &fixture.responder_trust()).await,
        NodeEvent::CapabilitiesUpdated
    ));

    let request_id = RequestId::from_bytes([0xc1; 16]);
    client
        .send_request(clipboard_request(request_id, b"network-control"))
        .unwrap();
    let NodeEvent::RequestDispatched(request) =
        eventually_node_event(&mut server, &fixture.initiator_trust()).await
    else {
        panic!("expected authorized control request");
    };
    let request_body = request.body().to_vec();

    server
        .send_response(request_id, ControlResponseResult::Success(b"ok".to_vec()))
        .unwrap();
    let NodeEvent::Response(response) =
        eventually_node_event(&mut client, &fixture.responder_trust()).await
    else {
        panic!("expected correlated control response");
    };
    let response_body = match response.result() {
        ControlResponseResult::Success(body) => body.clone(),
        ControlResponseResult::Error(error) => panic!("unexpected control error: {error:?}"),
    };

    assert!(
        server
            .subscribe_event(EventSubscription::new(
                clipboard_capability(),
                EventType::parse("clipboard.changed").unwrap(),
            ))
            .unwrap()
    );
    client
        .send_event(clipboard_event(
            EventId::from_bytes([0xc2; 16]),
            b"changed",
        ))
        .unwrap();
    let NodeEvent::Event(event) =
        eventually_node_event(&mut server, &fixture.initiator_trust()).await
    else {
        panic!("expected subscribed capability event");
    };
    let event_body = event.body().to_vec();

    client.shutdown();
    server.shutdown();
    drop((client, server));
    shutdown_parts(direct, client_transport, server_transport).await;

    ControlLifecycleEvidence {
        network_class,
        request_body,
        response_body,
        event_body,
    }
}

pub async fn exercise_stream_lifecycle(fixture: &AuthFixture) -> StreamLifecycleEvidence {
    let pair = authenticate_direct_pair(fixture, AuthAttempt::Normal)
        .await
        .expect("authenticated Iroh pair");
    let AuthenticatedIrohParts {
        direct,
        client_transport,
        server_transport,
        mut client_session,
        mut server_session,
    } = pair.into_parts();
    let authority = prepare_stream_authority(fixture, &mut client_session, &mut server_session);
    let mut sender = SimStreamRuntime::new(
        client_session,
        &client_transport,
        nonzero(STREAM_CAPACITY),
    )
    .unwrap();
    let mut receiver = SimStreamRuntime::new(
        server_session,
        &server_transport,
        nonzero(STREAM_CAPACITY),
    )
    .unwrap();
    receiver.register_operation(authority.operation).unwrap();

    let open = stream_open(
        &authority,
        authority.operation.id(),
        StreamId::from_bytes([0xc3; 16]),
    );
    let mut send = sender.open_uni(&open).unwrap();
    let stream_id = eventually_accept(
        &mut receiver,
        15,
        &authority.peer_trust,
        &authority.policy,
    )
    .await
    .expect("authorized stream should be admitted");
    send.try_send_chunk(b"payload".to_vec()).unwrap();
    let payload = eventually_receive(&mut receiver, stream_id).await;
    send.finish();
    eventually_finished(&mut receiver, stream_id).await;

    let unknown = stream_open(
        &authority,
        OperationId::from_bytes([0xc4; 32]),
        StreamId::from_bytes([0xc5; 16]),
    );
    let mut unknown_send = sender.open_uni(&unknown).unwrap();
    let unknown_operation_error = match eventually_accept(
        &mut receiver,
        15,
        &authority.peer_trust,
        &authority.policy,
    )
    .await
    {
        Err(SimStreamError::Admission(error)) => error,
        other => panic!("unexpected unknown-operation result: {other:?}"),
    };
    eventually_sender_closed(unknown_send.as_mut()).await;

    sender.shutdown();
    receiver.shutdown();
    drop((sender, receiver));
    shutdown_parts(direct, client_transport, server_transport).await;

    StreamLifecycleEvidence {
        payload,
        unknown_operation_error,
    }
}

pub async fn exercise_reconnect_lifecycle(fixture: &AuthFixture) -> ReconnectLifecycleEvidence {
    let first = authenticate_direct_pair(fixture, AuthAttempt::Normal)
        .await
        .expect("first authenticated Iroh pair");
    let old_binding = first.client_transport().channel_binding().bytes().to_vec();
    let old_session_id = first.client_session().context().unwrap().session_id();
    let old_proof = first.initiator_replay_proof();
    let AuthenticatedIrohParts {
        direct,
        client_transport,
        server_transport,
        mut client_session,
        mut server_session,
    } = first.into_parts();
    let old_authority = prepare_stream_authority(fixture, &mut client_session, &mut server_session);
    let old_operation = old_authority.operation;
    shutdown_parts(direct, client_transport, server_transport).await;

    let old_proof_error = match authenticate_direct_pair(
        fixture,
        AuthAttempt::ReplayInitiator(old_proof),
    )
    .await
    {
        Ok(pair) => {
            pair.shutdown().await;
            panic!("reconnect accepted old proof");
        }
        Err(rejected) => rejected.error(),
    };

    let reconnect = authenticate_direct_pair(fixture, AuthAttempt::Normal)
        .await
        .expect("fresh authenticated Iroh pair");
    let new_binding = reconnect.client_transport().channel_binding().bytes().to_vec();
    let new_session_id = reconnect.client_session().context().unwrap().session_id();
    let AuthenticatedIrohParts {
        direct,
        client_transport,
        server_transport,
        client_session,
        server_session,
    } = reconnect.into_parts();
    let protocol = client_session.context().unwrap().protocol_version();
    let mut client = SimNode::new(
        client_session,
        &client_transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Remote,
        nonzero(STATE_CAPACITY),
    )
    .unwrap();
    let mut server = SimNode::new(
        server_session,
        &server_transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Remote,
        nonzero(STATE_CAPACITY),
    )
    .unwrap();
    let pending_request_count = client.pending_request_count();
    let next_send_sequence = client.next_send_sequence();
    let expected_receive_sequence = client.expected_receive_sequence();
    let old_event = Event::system(
        EventId::from_bytes([0xc6; 16]),
        EventType::parse("crosslab.system.keepalive").unwrap(),
        Vec::new(),
    )
    .unwrap();
    let old_envelope = ControlEnvelope::new(
        protocol,
        old_session_id,
        0,
        EnvelopeBody::Event(old_event),
    );
    client_transport
        .try_send_control(encode_control_envelope(&old_envelope).unwrap())
        .unwrap();
    let old_session_error = eventually_node_dispatch_error(&mut server, &fixture.initiator_trust())
        .await;
    client.shutdown();
    server.shutdown();
    drop((client, server));
    shutdown_parts(direct, client_transport, server_transport).await;

    let operation_pair = authenticate_direct_pair(fixture, AuthAttempt::Normal)
        .await
        .expect("fresh pair for operation currentness");
    let AuthenticatedIrohParts {
        direct,
        client_transport,
        server_transport,
        mut client_session,
        mut server_session,
    } = operation_pair.into_parts();
    let current = prepare_stream_context(fixture, &mut client_session, &mut server_session);
    let mut sender = SimStreamRuntime::new(
        client_session,
        &client_transport,
        nonzero(STREAM_CAPACITY),
    )
    .unwrap();
    let mut receiver = SimStreamRuntime::new(
        server_session,
        &server_transport,
        nonzero(STREAM_CAPACITY),
    )
    .unwrap();
    let old_operation_id = old_operation.id();
    receiver.register_operation(old_operation).unwrap();
    let old_open = DataStreamOpen::new(
        current.session_id,
        StreamId::from_bytes([0xc7; 16]),
        old_operation_id,
        current.capability,
        current.version,
        current.operation_name,
        StreamDirection::SourceToDestination,
        0,
    );
    let mut old_send = sender.open_uni(&old_open).unwrap();
    let old_operation_error = match eventually_accept(
        &mut receiver,
        15,
        &current.peer_trust,
        &current.policy,
    )
    .await
    {
        Err(SimStreamError::Admission(error)) => error,
        other => panic!("unexpected old-operation result: {other:?}"),
    };
    eventually_sender_closed(old_send.as_mut()).await;
    sender.shutdown();
    receiver.shutdown();
    drop((sender, receiver));
    shutdown_parts(direct, client_transport, server_transport).await;

    ReconnectLifecycleEvidence {
        old_binding,
        new_binding,
        old_session_id,
        new_session_id,
        old_proof_error,
        old_session_error,
        old_operation_error,
        pending_request_count,
        next_send_sequence,
        expected_receive_sequence,
    }
}

pub async fn exercise_revocation_lifecycle(fixture: &AuthFixture) -> RevocationLifecycleEvidence {
    let pair = authenticate_direct_pair(fixture, AuthAttempt::Normal)
        .await
        .expect("authenticated Iroh pair");
    let AuthenticatedIrohParts {
        direct,
        client_transport,
        server_transport,
        mut client_session,
        mut server_session,
    } = pair.into_parts();
    let authority = prepare_stream_authority(fixture, &mut client_session, &mut server_session);
    let mut sender = SimStreamRuntime::new(
        client_session,
        &client_transport,
        nonzero(STREAM_CAPACITY),
    )
    .unwrap();
    let mut receiver = SimStreamRuntime::new(
        server_session,
        &server_transport,
        nonzero(STREAM_CAPACITY),
    )
    .unwrap();
    receiver.register_operation(authority.operation).unwrap();
    let open = stream_open(
        &authority,
        authority.operation.id(),
        StreamId::from_bytes([0xc8; 16]),
    );
    let mut send = sender.open_uni(&open).unwrap();
    let stream_id = eventually_accept(
        &mut receiver,
        15,
        &authority.peer_trust,
        &authority.policy,
    )
    .await
    .unwrap();

    let revoked_responder = fixture.revoked_responder();
    sender.apply_peer_revocation(&revoked_responder).unwrap();
    let local_state = sender.session().state();
    let stream_cancelled = matches!(
        eventually_receive_result(&mut receiver, stream_id).await,
        Err(SimStreamError::Receive(StreamReceiveError::Cancelled))
    );
    let remote_state = receiver.session().state();
    let after_revoke = b"after-revoke".to_vec();
    let sender_closed = send.try_send_chunk(after_revoke.clone())
        == Err(StreamSendError::Closed(after_revoke));

    drop((sender, receiver));
    shutdown_parts(direct, client_transport, server_transport).await;

    let reconnect_error = match authenticate_direct_pair_with_peer_trust(
        fixture,
        AuthAttempt::Normal,
        &revoked_responder,
        &fixture.initiator_trust(),
    )
    .await
    {
        Ok(pair) => {
            pair.shutdown().await;
            panic!("revoked peer reauthenticated");
        }
        Err(rejected) => rejected.error(),
    };

    RevocationLifecycleEvidence {
        local_state,
        remote_state,
        stream_cancelled,
        sender_closed,
        reconnect_error,
    }
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
            RuleId::from_bytes([0xc9; 32]),
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
    let context = prepare_stream_context(fixture, client_session, server_session);
    let authorization = AuthorizationContext::new(
        fixture.initiator_credential().device_id(),
        fixture.responder_credential().device_id(),
        context.session_id,
        context.capability.clone(),
        context.version,
        context.operation_name.clone(),
        TrustState::Trusted,
        context.peer_trust.trust_revision(),
        files_local_capability(),
        NetworkClass::Remote,
    );
    let grant = context.policy.evaluate(&authorization).into_grant().unwrap();
    let operation = AuthorizedOperation::issue(grant, 10, 20, UsePolicy::SingleStream).unwrap();

    StreamAuthority {
        capability: context.capability,
        version: context.version,
        operation_name: context.operation_name,
        operation,
        peer_trust: context.peer_trust,
        policy: context.policy,
        session_id: context.session_id,
    }
}

fn prepare_stream_context(
    fixture: &AuthFixture,
    client_session: &mut LogicalSession,
    server_session: &mut LogicalSession,
) -> StreamContext {
    let capability = files_capability();
    let version = CapabilityVersion::new(1, 0);
    let operation_name = OperationName::parse("send").unwrap();
    let local_capability = files_local_capability();
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
    let mut policy = PolicyState::new();
    policy
        .insert(PolicyRule::new(
            RuleId::from_bytes([0xca; 32]),
            fixture.initiator_credential().device_id(),
            capability.clone(),
            operation_name.clone(),
            RuleEffect::Allow,
        ))
        .unwrap();

    StreamContext {
        capability,
        version,
        operation_name,
        peer_trust: fixture.initiator_trust(),
        policy,
        session_id: server_session.context().unwrap().session_id(),
    }
}

fn files_local_capability() -> LocalCapability {
    LocalCapability::new(
        files_capability(),
        CapabilityVersionRange::new(1, 0, 0).unwrap(),
        true,
    )
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

async fn eventually_node_event(node: &mut SimNode<'_>, peer_trust: &TrustRecord) -> NodeEvent {
    timeout(WAIT, async {
        loop {
            match node.receive_one(peer_trust) {
                Ok(event) => return event,
                Err(NodeError::Receive(ControlReceiveError::Empty)) => {
                    tokio::task::yield_now().await;
                }
                Err(error) => panic!("node failed before expected event: {error:?}"),
            }
        }
    })
    .await
    .expect("node event was not delivered before timeout")
}

async fn eventually_node_dispatch_error(
    node: &mut SimNode<'_>,
    peer_trust: &TrustRecord,
) -> ControlDispatchError {
    timeout(WAIT, async {
        loop {
            match node.receive_one(peer_trust) {
                Err(NodeError::Dispatch(error)) => return error,
                Err(NodeError::Receive(ControlReceiveError::Empty)) => {
                    tokio::task::yield_now().await;
                }
                Ok(_) => tokio::task::yield_now().await,
                Err(error) => panic!("unexpected node error while waiting for dispatch: {error:?}"),
            }
        }
    })
    .await
    .expect("node dispatch error was not delivered before timeout")
}

async fn eventually_accept(
    runtime: &mut SimStreamRuntime<'_>,
    now: u64,
    peer_trust: &TrustRecord,
    policy: &PolicyState,
) -> Result<StreamId, SimStreamError> {
    timeout(WAIT, async {
        loop {
            match runtime.accept_one(now, peer_trust, policy) {
                Err(SimStreamError::Accept(StreamAcceptError::Empty)) => {
                    tokio::task::yield_now().await;
                }
                result => return result,
            }
        }
    })
    .await
    .expect("stream accept did not resolve before timeout")
}

async fn eventually_receive(runtime: &mut SimStreamRuntime<'_>, stream_id: StreamId) -> Vec<u8> {
    match eventually_receive_result(runtime, stream_id).await {
        Ok(chunk) => chunk,
        Err(error) => panic!("stream failed before payload arrived: {error:?}"),
    }
}

async fn eventually_receive_result(
    runtime: &mut SimStreamRuntime<'_>,
    stream_id: StreamId,
) -> Result<Vec<u8>, SimStreamError> {
    timeout(WAIT, async {
        loop {
            match runtime.try_receive_chunk(stream_id) {
                Err(SimStreamError::Receive(StreamReceiveError::Empty)) => {
                    tokio::task::yield_now().await;
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

async fn eventually_sender_closed(stream: &mut dyn TransportSendStream) {
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
    .expect("sender did not observe cancellation before timeout");
}

async fn shutdown_parts(
    direct: DirectPair,
    client_transport: IrohTransportConnection,
    server_transport: IrohTransportConnection,
) {
    timeout(WAIT, async {
        tokio::join!(client_transport.shutdown(), server_transport.shutdown());
        direct.shutdown().await;
    })
    .await
    .expect("Iroh lifecycle pair did not shut down before timeout");
}

fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
