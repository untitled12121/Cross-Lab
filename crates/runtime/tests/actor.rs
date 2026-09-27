use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
    time::Duration,
};

use crosslab_core::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, EventSubscription,
    IncomingUniStream, LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionHandshakeSide, SessionState, StreamAcceptError, StreamOpenError, StreamReceiveError,
    StreamSendError, TransportConnection, TransportReceiveStream, TransportSecurityClass,
    TransportSendStream,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    AuthorizationContext, AuthorizedOperation, CapabilityId, CapabilityVersion,
    CapabilityVersionRange, LocalCapability, NetworkClass, OperationName, PairingTrustTransition,
    PolicyState, RuleEffect, TransitionId, TrustRecord, TrustState, TrustTransition, UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlEnvelope, DataStreamOpen,
    EnvelopeBody, EventType, FeatureSet, ProtocolRange, ProtocolVersion, SessionClose,
    SessionCloseReason, StreamDirection, StreamId, encode_control_envelope,
    encode_data_stream_open,
};
use crosslab_runtime::{
    ConnectivityState, RuntimeActor, RuntimeActorConfig, RuntimeActorError, RuntimeActorSession,
    RuntimeActorStreamSendError, RuntimeNode,
};

#[derive(Clone)]
struct TestTransport {
    closed: Arc<Mutex<bool>>,
    inbound: Arc<Mutex<Vec<Vec<u8>>>>,
    incoming_streams: Arc<Mutex<VecDeque<IncomingUniStream>>>,
    outbound: Arc<Mutex<TestOutboundState>>,
    binding: ChannelBinding,
    metadata: ConnectionMetadata,
}

impl TestTransport {
    fn new(binding: [u8; 32]) -> Self {
        Self {
            closed: Arc::new(Mutex::new(false)),
            inbound: Arc::new(Mutex::new(Vec::new())),
            incoming_streams: Arc::new(Mutex::new(VecDeque::new())),
            outbound: Arc::new(Mutex::new(TestOutboundState::default())),
            binding: ChannelBinding::new("actor-test-binding", binding.to_vec()),
            metadata: ConnectionMetadata::new(None, None, Some(false)),
        }
    }

    fn push_inbound(&self, frame: Vec<u8>) {
        self.inbound.lock().unwrap().push(frame);
    }

    fn push_incoming_stream(&self, opening_frame: Vec<u8>, chunks: Vec<Vec<u8>>) {
        self.incoming_streams
            .lock()
            .unwrap()
            .push_back(IncomingUniStream::new(
                opening_frame,
                Box::new(TestReceiveStream {
                    chunks: chunks.into(),
                    cancelled: false,
                }),
            ));
    }

    fn make_next_send_full(&self) {
        self.outbound.lock().unwrap().full_next = true;
    }

    fn sent_chunks(&self) -> Vec<Vec<u8>> {
        self.outbound.lock().unwrap().chunks.clone()
    }

    fn opening_count(&self) -> usize {
        self.outbound.lock().unwrap().opening_frames.len()
    }

    fn finish_count(&self) -> usize {
        self.outbound.lock().unwrap().finished
    }

    fn cancel_count(&self) -> usize {
        self.outbound.lock().unwrap().cancelled
    }
}

#[derive(Default)]
struct TestOutboundState {
    opening_frames: Vec<Vec<u8>>,
    chunks: Vec<Vec<u8>>,
    full_next: bool,
    finished: usize,
    cancelled: usize,
}

struct TestSendStream {
    state: Arc<Mutex<TestOutboundState>>,
    closed: bool,
}

impl TransportSendStream for TestSendStream {
    fn try_send_chunk(&mut self, chunk: Vec<u8>) -> Result<(), StreamSendError> {
        if self.closed {
            return Err(StreamSendError::Closed(chunk));
        }
        let mut state = self.state.lock().unwrap();
        if state.full_next {
            state.full_next = false;
            return Err(StreamSendError::Full(chunk));
        }
        state.chunks.push(chunk);
        Ok(())
    }

    fn finish(&mut self) {
        if !self.closed {
            self.state.lock().unwrap().finished += 1;
            self.closed = true;
        }
    }

    fn cancel(&mut self) {
        if !self.closed {
            self.state.lock().unwrap().cancelled += 1;
            self.closed = true;
        }
    }
}

struct TestReceiveStream {
    chunks: VecDeque<Vec<u8>>,
    cancelled: bool,
}

impl TransportReceiveStream for TestReceiveStream {
    fn try_receive_chunk(&mut self) -> Result<Vec<u8>, StreamReceiveError> {
        if self.cancelled {
            return Err(StreamReceiveError::Cancelled);
        }
        self.chunks.pop_front().ok_or(StreamReceiveError::Finished)
    }

    fn cancel(&mut self) {
        self.cancelled = true;
        self.chunks.clear();
    }
}

impl TransportConnection for TestTransport {
    fn security_class(&self) -> TransportSecurityClass {
        TransportSecurityClass::InProcessTest
    }

    fn channel_binding(&self) -> &ChannelBinding {
        &self.binding
    }

    fn connection_metadata(&self) -> &ConnectionMetadata {
        &self.metadata
    }

    fn try_send_control(&self, frame: Vec<u8>) -> Result<(), ControlSendError> {
        if self.is_closed() {
            Err(ControlSendError::Closed(frame))
        } else {
            Ok(())
        }
    }

    fn try_receive_control(&self) -> Result<Vec<u8>, ControlReceiveError> {
        if self.is_closed() {
            return Err(ControlReceiveError::Closed);
        }
        let mut inbound = self.inbound.lock().unwrap();
        if inbound.is_empty() {
            Err(ControlReceiveError::Empty)
        } else {
            Ok(inbound.remove(0))
        }
    }

    fn try_open_uni_stream(
        &self,
        opening_frame: Vec<u8>,
    ) -> Result<Box<dyn crosslab_core::TransportSendStream>, StreamOpenError> {
        if self.is_closed() {
            return Err(StreamOpenError::Closed(opening_frame));
        }
        self.outbound
            .lock()
            .unwrap()
            .opening_frames
            .push(opening_frame);
        Ok(Box::new(TestSendStream {
            state: Arc::clone(&self.outbound),
            closed: false,
        }))
    }

    fn try_accept_uni_stream(&self) -> Result<IncomingUniStream, StreamAcceptError> {
        if self.is_closed() {
            return Err(StreamAcceptError::Closed);
        }
        self.incoming_streams
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(StreamAcceptError::Empty)
    }

    fn close(&self) {
        *self.closed.lock().unwrap() = true;
    }

    fn is_closed(&self) -> bool {
        *self.closed.lock().unwrap()
    }
}

struct Fixture {
    owner_id: OwnerId,
    authority: OwnerAuthorityState,
    issuer_key: SigningKey,
    local_key: SigningKey,
    peer_key: SigningKey,
    local_credential: DeviceCredential,
    peer_credential: DeviceCredential,
    peer_trust: TrustRecord,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x71; 32]);
        let root_key = SigningKey::from_secret_bytes([0x72; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x73; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let local_key = SigningKey::from_secret_bytes([0x74; 32]);
        let peer_key = SigningKey::from_secret_bytes([0x75; 32]);
        let local_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x76; 32]),
            &local_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let peer_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x77; 32]),
            &peer_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let peer_trust = PairingTrustTransition::issue(
            &peer_credential,
            TransitionId::from_bytes([0x78; 32]),
            [0x79; 32],
            &authority,
            &issuer_key,
        )
        .unwrap()
        .establish(&peer_credential, &authority)
        .unwrap();

        Self {
            owner_id,
            authority,
            issuer_key,
            local_key,
            peer_key,
            local_credential,
            peer_credential,
            peer_trust,
        }
    }

    fn revoked_peer(&self) -> TrustRecord {
        let mut trust = self.peer_trust;
        let transition = TrustTransition::issue_delegated_revocation(
            &trust,
            TransitionId::from_bytes([0x7a; 32]),
            &self.authority,
            AuthorityRole::DeviceSigning,
            &self.issuer_key,
        )
        .unwrap();
        transition
            .apply_delegated(&mut trust, &self.authority)
            .unwrap();
        trust
    }

    fn logical_session(&self, transport: &TestTransport, nonce: u8) -> LogicalSession {
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let initiator = SessionHandshakeSide::new(&self.local_credential, &ranges, &features);
        let responder = SessionHandshakeSide::new(&self.peer_credential, &ranges, &features);
        let transcript = SessionAuthTranscriptV1::new(
            self.owner_id,
            &self.local_credential,
            [nonce; 32],
            &self.peer_credential,
            [nonce.wrapping_add(1); 32],
            ProtocolVersion::new(1, 0),
            &[],
            transport.channel_binding().profile_id().as_bytes(),
            transport.channel_binding().bytes(),
        )
        .unwrap();
        let initiator_proof = transcript
            .create_proof(SessionAuthRole::Initiator, &self.local_key)
            .unwrap();
        let responder_proof = transcript
            .create_proof(SessionAuthRole::Responder, &self.peer_key)
            .unwrap();
        let mut session = LogicalSession::new();
        session
            .authenticate(SessionActivation::new(
                &self.authority,
                initiator,
                responder,
                SessionAuthRole::Initiator,
                &self.peer_trust,
                [nonce; 32],
                [nonce.wrapping_add(1); 32],
                transport.channel_binding(),
                TransportSecurityClass::InProcessTest,
                &initiator_proof,
                &responder_proof,
            ))
            .unwrap();
        session
    }

    fn actor_session(
        &self,
        binding: [u8; 32],
        nonce: u8,
    ) -> (RuntimeActorSession, Arc<TestTransport>) {
        let transport = Arc::new(TestTransport::new(binding));
        let session = self.logical_session(&transport, nonce);
        let node = RuntimeNode::new_owned(
            session,
            Arc::clone(&transport),
            PolicyState::new(),
            Vec::new(),
            NetworkClass::Local,
            NonZeroUsize::new(4).unwrap(),
        )
        .unwrap();

        (RuntimeActorSession::new(node, self.peer_trust), transport)
    }

    fn stream_actor_session(
        &self,
        binding: [u8; 32],
        nonce: u8,
    ) -> (RuntimeActorSession, Arc<TestTransport>, DataStreamOpen) {
        let transport = Arc::new(TestTransport::new(binding));
        let mut session = self.logical_session(&transport, nonce);
        let capability = CapabilityId::parse("files.transfer").unwrap();
        let version = CapabilityVersion::new(2, 0);
        let operation_name = OperationName::parse("receive").unwrap();
        let local = LocalCapability::new(
            capability.clone(),
            CapabilityVersionRange::new(2, 0, 0).unwrap(),
            true,
        );
        session
            .negotiate_capabilities(
                std::slice::from_ref(&local),
                &CapabilityAdvertisement::new(vec![
                    CapabilityAdvertisementEntry::new(capability.clone(), version, version, true)
                        .unwrap(),
                ])
                .unwrap(),
            )
            .unwrap();

        let mut policy = PolicyState::new();
        policy
            .set_rule_effect(
                self.peer_trust.device_id(),
                capability.clone(),
                operation_name.clone(),
                RuleEffect::Allow,
            )
            .unwrap();
        let context = session.context().unwrap();
        let grant = policy
            .evaluate(&AuthorizationContext::new(
                self.peer_trust.device_id(),
                context.local_device_id(),
                context.session_id(),
                capability.clone(),
                version,
                operation_name.clone(),
                self.peer_trust.state(),
                self.peer_trust.trust_revision(),
                local.clone(),
                NetworkClass::Local,
            ))
            .into_grant()
            .unwrap();
        let operation =
            AuthorizedOperation::issue(grant, 0, u64::MAX, UsePolicy::SingleStream).unwrap();
        let open = DataStreamOpen::new(
            context.session_id(),
            StreamId::from_bytes([0xa1; 16]),
            operation.id(),
            capability,
            version,
            operation_name,
            StreamDirection::SourceToDestination,
            0,
        );
        let mut node = RuntimeNode::new_owned(
            session,
            Arc::clone(&transport),
            policy,
            vec![local],
            NetworkClass::Local,
            NonZeroUsize::new(4).unwrap(),
        )
        .unwrap();
        node.register_stream_operation(operation).unwrap();

        (
            RuntimeActorSession::new(node, self.peer_trust),
            transport,
            open,
        )
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
}

fn config(capacity: usize) -> RuntimeActorConfig {
    RuntimeActorConfig::new(NonZeroUsize::new(capacity).unwrap())
}

#[test]
fn one_start_owns_one_actor_task_and_rejects_duplicate_start() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (first, first_transport) = fixture.actor_session([0x81; 32], 0x82);
        let (second, second_transport) = fixture.actor_session([0x83; 32], 0x84);
        let mut actor = RuntimeActor::new(config(4));

        actor.start(first).unwrap();
        assert!(actor.is_running());
        assert_eq!(actor.start(second), Err(RuntimeActorError::AlreadyRunning));

        drop(actor);
        tokio::task::yield_now().await;
        assert!(first_transport.is_closed());
        assert!(second_transport.is_closed());
    });
}

#[test]
fn stop_closes_active_transport_and_actor_task() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, transport) = fixture.actor_session([0x85; 32], 0x86);
        let mut actor = RuntimeActor::new(config(4));
        actor.start(session).unwrap();
        let mut status = actor.subscribe_status().unwrap();

        actor.stop().await.unwrap();

        assert!(!actor.is_running());
        assert!(transport.is_closed());
        let _ = status.changed().await;
        assert_eq!(
            status.borrow().connectivity(),
            ConnectivityState::Disconnected
        );
        assert_eq!(status.borrow().session_id(), None);
    });
}

#[test]
fn network_loss_drops_session_authority_before_reconnect() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, transport) = fixture.actor_session([0x87; 32], 0x88);
        let mut actor = RuntimeActor::new(config(4));
        actor.start(session).unwrap();
        let mut status = actor.subscribe_status().unwrap();

        actor.try_network_lost().unwrap();
        status.changed().await.unwrap();

        assert!(transport.is_closed());
        assert_eq!(
            status.borrow().connectivity(),
            ConnectivityState::Disconnected
        );
        assert_eq!(status.borrow().session_state(), SessionState::Closed);
        assert_eq!(status.borrow().session_id(), None);
        actor.stop().await.unwrap();
    });
}

#[test]
fn control_readiness_drives_inbound_events_without_polling() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, transport) = fixture.actor_session([0x99; 32], 0x9a);
        let session_id = session.session_id();
        let (ready_tx, ready_rx) = tokio::sync::watch::channel(0_u64);
        let mut actor = RuntimeActor::new(config(4));
        actor.start(session.with_control_ready(ready_rx)).unwrap();
        let mut events = actor.take_events().unwrap();

        let close = ControlEnvelope::new(
            ProtocolVersion::new(1, 0),
            session_id,
            0,
            EnvelopeBody::SessionClose(SessionClose::new(SessionCloseReason::Normal, None)),
        );
        transport.push_inbound(encode_control_envelope(&close).unwrap());
        ready_tx.send_replace(1);

        let event = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("runtime actor should react to readiness")
            .expect("runtime actor event channel should stay open");
        assert!(matches!(
            event,
            crosslab_runtime::NodeEvent::SessionClosed(SessionCloseReason::Normal)
        ));
        assert!(transport.is_closed());
    });
}

#[test]
fn stream_readiness_drives_inbound_stream_events_without_polling() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, transport, open) = fixture.stream_actor_session([0x9b; 32], 0x9c);
        let (ready_tx, ready_rx) = tokio::sync::watch::channel(0_u64);
        let mut actor = RuntimeActor::new(config(4));
        actor.start(session.with_stream_ready(ready_rx)).unwrap();
        let mut events = actor.take_events().unwrap();

        transport.push_incoming_stream(
            encode_data_stream_open(&open).unwrap(),
            vec![b"private-stream-payload".to_vec()],
        );
        ready_tx.send_replace(1);

        let opened = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("runtime actor should react to stream readiness")
            .expect("runtime actor event channel should stay open");
        let chunk = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("runtime actor should drain stream payload")
            .expect("runtime actor event channel should stay open");
        let finished = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("runtime actor should drain stream completion")
            .expect("runtime actor event channel should stay open");

        assert!(matches!(
            opened,
            crosslab_runtime::NodeEvent::Stream(crosslab_runtime::RuntimeStreamEvent::Opened(_))
        ));
        assert!(matches!(
            chunk,
            crosslab_runtime::NodeEvent::Stream(crosslab_runtime::RuntimeStreamEvent::Chunk(ref chunk))
                if chunk.bytes() == b"private-stream-payload"
        ));
        assert!(!format!("{chunk:?}").contains("private-stream-payload"));
        assert!(matches!(
            finished,
            crosslab_runtime::NodeEvent::Stream(crosslab_runtime::RuntimeStreamEvent::Finished(_))
        ));

        actor.stop().await.unwrap();
    });
}

#[test]
fn stream_authority_commands_issue_and_cancel_exact_operation() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, _, _) = fixture.stream_actor_session([0x9f; 32], 0xa0);
        let mut actor = RuntimeActor::new(config(4));
        actor.start(session).unwrap();

        let operation_id = actor
            .issue_stream_operation(
                CapabilityId::parse("files.transfer").unwrap(),
                CapabilityVersion::new(2, 0),
                OperationName::parse("receive").unwrap(),
                Duration::from_secs(30),
                UsePolicy::SingleStream,
            )
            .await
            .unwrap();
        actor.cancel_stream_operation(operation_id).await.unwrap();
        assert_eq!(
            actor.cancel_stream_operation(operation_id).await,
            Err(RuntimeActorError::OperationRejected)
        );

        actor.stop().await.unwrap();
    });
}

#[test]
fn data_stream_commands_preserve_chunk_ownership_under_backpressure() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, transport, mut open) = fixture.stream_actor_session([0xa1; 32], 0xa2);
        open = DataStreamOpen::new(
            open.session_id(),
            StreamId::from_bytes([0xa3; 16]),
            open.operation_id(),
            open.capability_id().clone(),
            open.capability_version(),
            open.operation_name().clone(),
            StreamDirection::SourceToDestination,
            0,
        );
        let mut actor = RuntimeActor::new(config(4));
        actor.start(session).unwrap();

        let stream_id = actor.open_data_stream(open.clone()).await.unwrap();
        assert_eq!(stream_id, StreamId::from_bytes([0xa3; 16]));

        let chunk = b"private-file-chunk".to_vec();
        transport.make_next_send_full();
        let error = actor
            .send_stream_chunk(stream_id, chunk.clone())
            .await
            .unwrap_err();
        assert!(!format!("{error:?}").contains("private-file-chunk"));
        assert!(matches!(
            error,
            RuntimeActorStreamSendError::Stream(StreamSendError::Full(returned))
                if returned == chunk
        ));

        actor
            .send_stream_chunk(stream_id, chunk.clone())
            .await
            .unwrap();
        assert_eq!(transport.sent_chunks(), vec![chunk]);

        actor.finish_data_stream(stream_id).await.unwrap();
        assert_eq!(transport.finish_count(), 1);
        assert_eq!(
            actor.finish_data_stream(stream_id).await,
            Err(RuntimeActorError::StreamRejected)
        );

        let second_id = StreamId::from_bytes([0xa4; 16]);
        let second = DataStreamOpen::new(
            open.session_id(),
            second_id,
            open.operation_id(),
            open.capability_id().clone(),
            open.capability_version(),
            open.operation_name().clone(),
            StreamDirection::SourceToDestination,
            0,
        );
        assert_eq!(actor.open_data_stream(second).await.unwrap(), second_id);
        actor.cancel_outbound_stream(second_id).await.unwrap();
        assert_eq!(transport.opening_count(), 2);
        assert_eq!(transport.cancel_count(), 1);

        actor.stop().await.unwrap();
    });
}

#[test]
fn stream_chunk_actor_queue_backpressure_returns_owned_buffer() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, transport, mut open) = fixture.stream_actor_session([0xa5; 32], 0xa6);
        let stream_id = StreamId::from_bytes([0xa7; 16]);
        open = DataStreamOpen::new(
            open.session_id(),
            stream_id,
            open.operation_id(),
            open.capability_id().clone(),
            open.capability_version(),
            open.operation_name().clone(),
            StreamDirection::SourceToDestination,
            0,
        );
        let mut actor = RuntimeActor::new(config(1));
        actor.start(session).unwrap();
        actor.open_data_stream(open).await.unwrap();

        actor.try_network_lost().unwrap();
        let chunk = b"queue-owned-file-chunk".to_vec();
        let error = actor
            .send_stream_chunk(stream_id, chunk.clone())
            .await
            .unwrap_err();

        assert!(!format!("{error:?}").contains("queue-owned-file-chunk"));
        assert!(matches!(
            error,
            RuntimeActorStreamSendError::QueueFull(returned) if returned == chunk
        ));

        drop(actor);
        tokio::task::yield_now().await;
        assert!(transport.is_closed());
    });
}

#[test]
fn event_subscription_commands_are_actor_owned_and_bounded() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, _) = fixture.actor_session([0x9d; 32], 0x9e);
        let mut actor = RuntimeActor::new(config(1));
        actor.start(session).unwrap();

        let subscription = EventSubscription::new(
            CapabilityId::parse("files.transfer").unwrap(),
            EventType::parse("files.transfer.result").unwrap(),
        );

        assert!(actor.subscribe_event(subscription.clone()).await.unwrap());
        assert!(!actor.subscribe_event(subscription.clone()).await.unwrap());
        assert!(actor.unsubscribe_event(subscription.clone()).await.unwrap());
        assert!(!actor.unsubscribe_event(subscription).await.unwrap());

        actor.stop().await.unwrap();
    });
}

#[test]
fn policy_replacement_is_applied_by_the_actor() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, _) = fixture.actor_session([0x97; 32], 0x98);
        let mut actor = RuntimeActor::new(config(4));
        actor.start(session).unwrap();

        let mut policy = PolicyState::new();
        policy
            .set_rule_effect(
                fixture.peer_trust.device_id(),
                CapabilityId::parse("files.transfer").unwrap(),
                OperationName::parse("receive").unwrap(),
                RuleEffect::Allow,
            )
            .unwrap();

        assert!(actor.replace_policy(policy.clone()).await.unwrap());
        assert!(!actor.replace_policy(policy).await.unwrap());
        actor.stop().await.unwrap();
    });
}

#[test]
fn reconnect_requires_a_fresh_authenticated_session() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (first, first_transport) = fixture.actor_session([0x89; 32], 0x8a);
        let first_id = first.session_id();
        let mut actor = RuntimeActor::new(config(4));
        actor.start(first).unwrap();
        let mut status = actor.subscribe_status().unwrap();
        actor.try_network_lost().unwrap();
        status.changed().await.unwrap();

        let (stale, stale_transport) = fixture.actor_session([0x89; 32], 0x8a);
        assert_eq!(stale.session_id(), first_id);
        assert_eq!(
            actor.reconnect(stale).await,
            Err(RuntimeActorError::StaleSession)
        );
        assert!(stale_transport.is_closed());

        let (fresh, fresh_transport) = fixture.actor_session([0x8b; 32], 0x8c);
        let fresh_id = fresh.session_id();
        assert_ne!(fresh_id, first_id);
        actor.reconnect(fresh).await.unwrap();
        status.changed().await.unwrap();

        assert!(first_transport.is_closed());
        assert!(!fresh_transport.is_closed());
        assert_eq!(status.borrow().session_id(), Some(fresh_id));
        assert_eq!(status.borrow().connectivity(), ConnectivityState::Connected);
        actor.stop().await.unwrap();
    });
}

#[test]
fn peer_revocation_closes_authority_and_publishes_revoked_status() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, transport) = fixture.actor_session([0x95; 32], 0x96);
        let mut actor = RuntimeActor::new(config(4));
        actor.start(session).unwrap();
        let mut status = actor.subscribe_status().unwrap();

        actor.revoke_peer(fixture.revoked_peer()).await.unwrap();
        status.changed().await.unwrap();

        assert!(transport.is_closed());
        assert_eq!(status.borrow().trust_state(), TrustState::Revoked);
        assert_eq!(
            status.borrow().connectivity(),
            ConnectivityState::Disconnected
        );
        assert_eq!(status.borrow().session_state(), SessionState::Closed);
        assert_eq!(status.borrow().session_id(), None);
        actor.stop().await.unwrap();
    });
}

#[test]
fn command_queue_is_bounded() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, _) = fixture.actor_session([0x8d; 32], 0x8e);
        let mut actor = RuntimeActor::new(config(1));
        actor.start(session).unwrap();

        actor.try_network_lost().unwrap();
        assert_eq!(
            actor.try_network_lost(),
            Err(RuntimeActorError::CommandQueueFull)
        );

        tokio::task::yield_now().await;
        actor.stop().await.unwrap();
    });
}

#[test]
fn slow_status_subscriber_only_observes_latest_snapshot() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (first, _) = fixture.actor_session([0x8f; 32], 0x90);
        let mut actor = RuntimeActor::new(config(4));
        actor.start(first).unwrap();
        let status = actor.subscribe_status().unwrap();

        actor.try_network_lost().unwrap();
        tokio::task::yield_now().await;
        let (fresh, _) = fixture.actor_session([0x91; 32], 0x92);
        actor.reconnect(fresh).await.unwrap();
        actor.try_network_lost().unwrap();
        tokio::task::yield_now().await;

        assert_eq!(
            status.borrow().connectivity(),
            ConnectivityState::Disconnected
        );
        assert_eq!(status.borrow().session_id(), None);
        actor.stop().await.unwrap();
    });
}

#[test]
fn dropping_actor_handle_closes_transport_without_orphaning_authority() {
    runtime().block_on(async {
        let fixture = Fixture::new();
        let (session, transport) = fixture.actor_session([0x93; 32], 0x94);
        let mut actor = RuntimeActor::new(config(4));
        actor.start(session).unwrap();

        drop(actor);
        for _ in 0..3 {
            tokio::task::yield_now().await;
        }

        assert!(transport.is_closed());
    });
}
