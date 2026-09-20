use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use crosslab_core::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, EventSubscription,
    IncomingUniStream, LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionError, SessionHandshakeSide, SessionState, StreamAcceptError, StreamOpenError,
    TransportConnection, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord, RootSuccessor,
};
use crosslab_policy::{
    CapabilityId, NetworkClass, OperationName, PairingTrustTransition, PolicyState, TransitionId,
    TrustRecord, TrustState, TrustTransition,
};
use crosslab_protocol::{
    ControlRequest, EventType, FeatureSet, ProtocolRange, ProtocolVersion, RequestId, RetryClass,
};

use crate::{ConnectivityState, NodeError, RuntimeNode};

struct TestTransportState {
    inbound: VecDeque<Vec<u8>>,
    outbound: VecDeque<Vec<u8>>,
    closed: bool,
}

struct TestTransport {
    state: Arc<Mutex<TestTransportState>>,
    binding: ChannelBinding,
    metadata: ConnectionMetadata,
}

impl TestTransport {
    fn new(binding: [u8; 32]) -> Self {
        Self {
            state: Arc::new(Mutex::new(TestTransportState {
                inbound: VecDeque::new(),
                outbound: VecDeque::new(),
                closed: false,
            })),
            binding: ChannelBinding::new("secret-binding-profile", binding.to_vec()),
            metadata: ConnectionMetadata::new(
                Some("secret-local-endpoint".into()),
                Some("secret-remote-endpoint".into()),
                Some(false),
            ),
        }
    }

    fn disconnect_now(&self) {
        self.state.lock().unwrap().closed = true;
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
        let mut state = self.state.lock().unwrap();
        if state.closed {
            return Err(ControlSendError::Closed(frame));
        }
        state.outbound.push_back(frame);
        Ok(())
    }

    fn try_receive_control(&self) -> Result<Vec<u8>, ControlReceiveError> {
        let mut state = self.state.lock().unwrap();
        if state.closed {
            return Err(ControlReceiveError::Closed);
        }
        state.inbound.pop_front().ok_or(ControlReceiveError::Empty)
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
        self.state.lock().unwrap().closed = true;
    }

    fn is_closed(&self) -> bool {
        self.state.lock().unwrap().closed
    }
}

struct Fixture {
    owner_id: OwnerId,
    root_key: SigningKey,
    authority: OwnerAuthorityState,
    local_key: SigningKey,
    peer_key: SigningKey,
    local_credential: DeviceCredential,
    peer_credential: DeviceCredential,
    peer_trust: TrustRecord,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x51; 32]);
        let root_key = SigningKey::from_secret_bytes([0x52; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x53; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let local_key = SigningKey::from_secret_bytes([0x54; 32]);
        let peer_key = SigningKey::from_secret_bytes([0x55; 32]);
        let local_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x56; 32]),
            &local_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let peer_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x57; 32]),
            &peer_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let peer_trust = PairingTrustTransition::issue(
            &peer_credential,
            TransitionId::from_bytes([0x58; 32]),
            [0x59; 32],
            &authority,
            &issuer_key,
        )
        .unwrap()
        .establish(&peer_credential, &authority)
        .unwrap();

        Self {
            owner_id,
            root_key,
            authority,
            local_key,
            peer_key,
            local_credential,
            peer_credential,
            peer_trust,
        }
    }

    fn active_session(&self, transport: &TestTransport) -> LogicalSession {
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let initiator = SessionHandshakeSide::new(&self.local_credential, &ranges, &features);
        let responder = SessionHandshakeSide::new(&self.peer_credential, &ranges, &features);
        let transcript = SessionAuthTranscriptV1::new(
            self.owner_id,
            &self.local_credential,
            [0x5a; 32],
            &self.peer_credential,
            [0x5b; 32],
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
                [0x5a; 32],
                [0x5b; 32],
                transport.channel_binding(),
                TransportSecurityClass::InProcessTest,
                &initiator_proof,
                &responder_proof,
            ))
            .unwrap();
        session
    }

    fn revoked_peer(&self) -> TrustRecord {
        let mut revoked = self.peer_trust;
        let transition = TrustTransition::issue_root_revocation(
            &revoked,
            TransitionId::from_bytes([0x5c; 32]),
            &self.authority,
            &self.root_key,
        )
        .unwrap();
        transition
            .apply_root(&mut revoked, &self.authority)
            .unwrap();
        revoked
    }

    fn rotate_device_signing(&mut self) {
        let replacement = SigningKey::from_secret_bytes([0x5d; 32]);
        let delegation = AuthorityDelegation::issue(
            self.owner_id,
            AuthorityRole::DeviceSigning,
            &replacement,
            1,
            &self.root_key,
        );
        self.authority.accept_delegation(delegation).unwrap();
    }

    fn rotate_owner_root(&mut self) {
        let replacement = SigningKey::from_secret_bytes([0x5e; 32]);
        let successor =
            RootSuccessor::issue(self.authority.root(), &self.root_key, &replacement).unwrap();
        self.authority.accept_root_successor(&successor).unwrap();
    }
}

fn request(byte: u8) -> ControlRequest {
    ControlRequest::new(
        RequestId::from_bytes([byte; 16]),
        CapabilityId::parse("files.transfer").unwrap(),
        crosslab_policy::CapabilityVersion::new(1, 0),
        OperationName::parse("send").unwrap(),
        RetryClass::NonRetryable,
        vec![byte],
    )
}

fn subscription(event_type: &str) -> EventSubscription {
    EventSubscription::new(
        CapabilityId::parse("files.transfer").unwrap(),
        EventType::parse(event_type).unwrap(),
    )
}

#[test]
fn runtime_requires_active_session() {
    let transport = TestTransport::new([0x60; 32]);
    let result = RuntimeNode::new(
        LogicalSession::new(),
        &transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(4).unwrap(),
    );

    assert!(matches!(
        result,
        Err(NodeError::Session(SessionError::InvalidState))
    ));
}

#[test]
fn dispatcher_request_state_remains_bounded() {
    let fixture = Fixture::new();
    let transport = TestTransport::new([0x61; 32]);
    let session = fixture.active_session(&transport);
    let mut runtime = RuntimeNode::new(
        session,
        &transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();

    runtime.send_request(request(0x62)).unwrap();
    assert_eq!(runtime.pending_request_count(), 1);
    assert!(matches!(
        runtime.send_request(request(0x63)),
        Err(NodeError::Dispatch(
            crosslab_core::ControlDispatchError::ResourceLimit
        ))
    ));
}

#[test]
fn dispatcher_event_subscriptions_remain_bounded() {
    let fixture = Fixture::new();
    let transport = TestTransport::new([0x6a; 32]);
    let session = fixture.active_session(&transport);
    let mut runtime = RuntimeNode::new(
        session,
        &transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    let first = subscription("files.progress");

    assert!(runtime.subscribe_event(first.clone()).unwrap());
    assert!(!runtime.subscribe_event(first).unwrap());
    assert!(matches!(
        runtime.subscribe_event(subscription("files.completed")),
        Err(NodeError::Dispatch(
            crosslab_core::ControlDispatchError::ResourceLimit
        ))
    ));
}

#[test]
fn transport_loss_closes_session_authority() {
    let fixture = Fixture::new();
    let transport = TestTransport::new([0x64; 32]);
    let session = fixture.active_session(&transport);
    let mut runtime = RuntimeNode::new(
        session,
        &transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(4).unwrap(),
    )
    .unwrap();
    runtime.send_request(request(0x65)).unwrap();

    transport.disconnect_now();
    assert!(matches!(
        runtime.receive_one(&fixture.peer_trust),
        Err(NodeError::Receive(ControlReceiveError::Closed))
    ));
    assert_eq!(runtime.session().state(), SessionState::Closed);
    assert_eq!(runtime.pending_request_count(), 0);
}

#[test]
fn peer_revocation_clears_dispatcher_state_and_transport() {
    let fixture = Fixture::new();
    let transport = TestTransport::new([0x66; 32]);
    let session = fixture.active_session(&transport);
    let mut runtime = RuntimeNode::new(
        session,
        &transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(4).unwrap(),
    )
    .unwrap();
    runtime.send_request(request(0x67)).unwrap();

    runtime
        .apply_peer_revocation(&fixture.revoked_peer())
        .unwrap();
    assert_eq!(runtime.session().state(), SessionState::Closed);
    assert_eq!(runtime.pending_request_count(), 0);
    assert!(transport.is_closed());
}

#[test]
fn device_signing_authority_replacement_fails_closed() {
    let mut fixture = Fixture::new();
    let transport = TestTransport::new([0x68; 32]);
    let session = fixture.active_session(&transport);
    let mut runtime = RuntimeNode::new(
        session,
        &transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(4).unwrap(),
    )
    .unwrap();
    runtime.send_request(request(0x6b)).unwrap();
    fixture.rotate_device_signing();

    assert!(matches!(
        runtime.revalidate_authority(&fixture.authority),
        Err(NodeError::Session(
            SessionError::DeviceSigningAuthorityChanged
        ))
    ));
    assert_eq!(runtime.session().state(), SessionState::Closed);
    assert_eq!(runtime.pending_request_count(), 0);
    assert!(transport.is_closed());
}

#[test]
fn owner_root_replacement_fails_closed() {
    let mut fixture = Fixture::new();
    let transport = TestTransport::new([0x6c; 32]);
    let session = fixture.active_session(&transport);
    let mut runtime = RuntimeNode::new(
        session,
        &transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(4).unwrap(),
    )
    .unwrap();
    runtime.send_request(request(0x6d)).unwrap();
    fixture.rotate_owner_root();

    assert!(matches!(
        runtime.revalidate_authority(&fixture.authority),
        Err(NodeError::Session(SessionError::OwnerAuthorityChanged))
    ));
    assert_eq!(runtime.session().state(), SessionState::Closed);
    assert_eq!(runtime.pending_request_count(), 0);
    assert!(transport.is_closed());
}

#[test]
fn status_snapshot_exposes_only_safe_summary_data() {
    let fixture = Fixture::new();
    let transport = TestTransport::new([0x69; 32]);
    let session = fixture.active_session(&transport);
    let owner_id = session.context().unwrap().owner_id();
    let session_id = session.context().unwrap().session_id();
    let local_device_id = session.context().unwrap().local_device_id();
    let peer_device_id = session.context().unwrap().peer_device_id();
    let runtime = RuntimeNode::new(
        session,
        &transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(4).unwrap(),
    )
    .unwrap();

    let status = runtime.status(&fixture.peer_trust);
    assert_eq!(status.owner_id(), Some(owner_id));
    assert_eq!(status.local_device_id(), Some(local_device_id));
    assert_eq!(status.peer_device_id(), Some(peer_device_id));
    assert_eq!(status.session_id(), Some(session_id));
    assert_eq!(status.session_state(), SessionState::Active);
    assert_eq!(status.connectivity(), ConnectivityState::Connected);
    assert_eq!(status.trust_state(), TrustState::Trusted);
    assert_eq!(status.protocol_version(), Some(ProtocolVersion::new(1, 0)));

    let rendered = format!("{status:?}");
    for forbidden in [
        "secret-binding-profile",
        "secret-local-endpoint",
        "secret-remote-endpoint",
    ] {
        assert!(!rendered.contains(forbidden));
    }
}

#[test]
fn inactive_status_drops_session_id() {
    let fixture = Fixture::new();
    let transport = TestTransport::new([0x6e; 32]);
    let session = fixture.active_session(&transport);
    let mut runtime = RuntimeNode::new(
        session,
        &transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(4).unwrap(),
    )
    .unwrap();
    assert!(runtime.status(&fixture.peer_trust).session_id().is_some());

    runtime.shutdown();
    let status = runtime.status(&fixture.peer_trust);
    assert_eq!(status.session_id(), None);
    assert_eq!(status.session_state(), SessionState::Closed);
    assert_eq!(status.connectivity(), ConnectivityState::Disconnected);
}
