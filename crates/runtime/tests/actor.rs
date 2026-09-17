use std::{num::NonZeroUsize, sync::{Arc, Mutex}};

use crosslab_core::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, IncomingUniStream,
    LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1, SessionHandshakeSide,
    SessionState, StreamAcceptError, StreamOpenError, TransportConnection, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    NetworkClass, PairingTrustTransition, PolicyState, TransitionId, TrustRecord,
};
use crosslab_protocol::{FeatureSet, ProtocolRange, ProtocolVersion};
use crosslab_runtime::{
    ConnectivityState, RuntimeActor, RuntimeActorConfig, RuntimeActorError, RuntimeActorSession,
    RuntimeNode,
};

#[derive(Clone)]
struct TestTransport {
    closed: Arc<Mutex<bool>>,
    binding: ChannelBinding,
    metadata: ConnectionMetadata,
}

impl TestTransport {
    fn new(binding: [u8; 32]) -> Self {
        Self {
            closed: Arc::new(Mutex::new(false)),
            binding: ChannelBinding::new("actor-test-binding", binding.to_vec()),
            metadata: ConnectionMetadata::new(None, None, Some(false)),
        }
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
            Err(ControlReceiveError::Closed)
        } else {
            Err(ControlReceiveError::Empty)
        }
    }

    fn try_open_uni_stream(
        &self,
        opening_frame: Vec<u8>,
    ) -> Result<Box<dyn crosslab_core::TransportSendStream>, StreamOpenError> {
        Err(StreamOpenError::Closed(opening_frame))
    }

    fn try_accept_uni_stream(&self) -> Result<IncomingUniStream, StreamAcceptError> {
        Err(StreamAcceptError::Closed)
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
            local_key,
            peer_key,
            local_credential,
            peer_credential,
            peer_trust,
        }
    }

    fn actor_session(
        &self,
        binding: [u8; 32],
        nonce: u8,
    ) -> (RuntimeActorSession<TestTransport>, Arc<TestTransport>) {
        let transport = Arc::new(TestTransport::new(binding));
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
        let node = RuntimeNode::new(
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
        assert_eq!(status.borrow().connectivity(), ConnectivityState::Disconnected);
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
        assert_eq!(status.borrow().connectivity(), ConnectivityState::Disconnected);
        assert_eq!(status.borrow().session_state(), SessionState::Closed);
        assert_eq!(status.borrow().session_id(), None);
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

        assert_eq!(status.borrow().connectivity(), ConnectivityState::Disconnected);
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
