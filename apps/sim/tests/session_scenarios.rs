use std::num::NonZeroUsize;

use crosslab_core::{
    ControlDispatchError, LogicalSession, SessionActivation, SessionAuthRole,
    SessionAuthTranscriptV1, SessionHandshakeSide, SessionState, TransportConnection,
    TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, DecisionReason, LocalCapability,
    OperationName, PolicyRule, PolicyState, RuleEffect, RuleId, TransitionId, TrustRecord,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlEnvelope, ControlRequest,
    ControlResponseResult, EnvelopeBody, Event, EventId, EventType, FeatureSet, ProtocolRange,
    ProtocolVersion, RequestId, RetryClass, SequenceError, SessionClose, SessionCloseReason,
    encode_control_envelope,
};
use crosslab_sim::{
    node::{NodeError, NodeEvent, SimNode},
    transport::MemoryTransportPair,
};

const CONTROL_CAPACITY: usize = 16;
const STATE_CAPACITY: usize = 16;

struct Fixture {
    owner_id: OwnerId,
    root: OwnerRootRecord,
    delegation: AuthorityDelegation,
    initiator_key: SigningKey,
    responder_key: SigningKey,
    initiator_credential: DeviceCredential,
    responder_credential: DeviceCredential,
    initiator_trust: TrustRecord,
    responder_trust: TrustRecord,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x30; 32]);
        let root_key = SigningKey::from_secret_bytes([0x31; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x32; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let initiator_key = SigningKey::from_secret_bytes([0x33; 32]);
        let responder_key = SigningKey::from_secret_bytes([0x34; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x35; 32]),
            &initiator_key,
            2,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x36; 32]),
            &responder_key,
            5,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let initiator_trust = TrustRecord::trusted(
            owner_id,
            initiator_credential.device_id(),
            initiator_credential.credential_epoch(),
            TransitionId::from_bytes([0x37; 32]),
        );
        let responder_trust = TrustRecord::trusted(
            owner_id,
            responder_credential.device_id(),
            responder_credential.credential_epoch(),
            TransitionId::from_bytes([0x38; 32]),
        );

        Self {
            owner_id,
            root,
            delegation,
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
            initiator_trust,
            responder_trust,
        }
    }

    fn protocol_ranges() -> [ProtocolRange; 1] {
        [ProtocolRange::new(1, 0, 0).unwrap()]
    }

    fn features() -> FeatureSet {
        FeatureSet::new(&[1], &[]).unwrap()
    }

    fn local_capabilities() -> Vec<LocalCapability> {
        vec![LocalCapability::new(
            CapabilityId::parse("clipboard.write").unwrap(),
            CapabilityVersionRange::new(1, 0, 0).unwrap(),
            true,
        )]
    }

    fn advertisement() -> CapabilityAdvertisement {
        CapabilityAdvertisement::new(vec![
            CapabilityAdvertisementEntry::new(
                CapabilityId::parse("clipboard.write").unwrap(),
                CapabilityVersion::new(1, 0),
                CapabilityVersion::new(1, 0),
                true,
            )
            .unwrap(),
        ])
        .unwrap()
    }

    fn sessions(
        &self,
        pair: &MemoryTransportPair,
        pre_negotiate_capabilities: bool,
    ) -> (LogicalSession, LogicalSession) {
        let ranges = Self::protocol_ranges();
        let features = Self::features();
        let (endpoint_a, _) = pair.endpoints();
        let binding = endpoint_a.channel_binding();
        let transcript = SessionAuthTranscriptV1::new(
            self.owner_id,
            &self.initiator_credential,
            [0x39; 32],
            &self.responder_credential,
            [0x3a; 32],
            ProtocolVersion::new(1, 0),
            &[1],
            binding.profile_id().as_bytes(),
            binding.bytes(),
        )
        .unwrap();
        let initiator_proof = transcript
            .create_proof(SessionAuthRole::Initiator, &self.initiator_key)
            .unwrap();
        let responder_proof = transcript
            .create_proof(SessionAuthRole::Responder, &self.responder_key)
            .unwrap();
        let initiator = SessionHandshakeSide::new(
            &self.initiator_credential,
            &self.delegation,
            &ranges,
            &features,
        );
        let responder = SessionHandshakeSide::new(
            &self.responder_credential,
            &self.delegation,
            &ranges,
            &features,
        );

        let mut session_a = LogicalSession::new();
        session_a
            .authenticate(SessionActivation::new(
                &self.root,
                initiator,
                responder,
                SessionAuthRole::Initiator,
                &self.responder_trust,
                [0x39; 32],
                [0x3a; 32],
                binding,
                TransportSecurityClass::InProcessTest,
                &initiator_proof,
                &responder_proof,
            ))
            .unwrap();

        let mut session_b = LogicalSession::new();
        session_b
            .authenticate(SessionActivation::new(
                &self.root,
                initiator,
                responder,
                SessionAuthRole::Responder,
                &self.initiator_trust,
                [0x39; 32],
                [0x3a; 32],
                binding,
                TransportSecurityClass::InProcessTest,
                &initiator_proof,
                &responder_proof,
            ))
            .unwrap();

        if pre_negotiate_capabilities {
            let local = Self::local_capabilities();
            let peer = Self::advertisement();
            session_a.negotiate_capabilities(&local, &peer).unwrap();
            session_b.negotiate_capabilities(&local, &peer).unwrap();
        }

        (session_a, session_b)
    }

    fn allow_write_policy(&self) -> PolicyState {
        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([0x3b; 32]),
                self.initiator_credential.device_id(),
                CapabilityId::parse("clipboard.write").unwrap(),
                OperationName::parse("set").unwrap(),
                RuleEffect::Allow,
            ))
            .unwrap();
        policy
    }
}

fn transport_pair() -> MemoryTransportPair {
    MemoryTransportPair::new(NonZeroUsize::new(CONTROL_CAPACITY).unwrap(), [0x40; 32])
}

fn request(request_id: RequestId, retry_class: RetryClass, body: &[u8]) -> ControlRequest {
    ControlRequest::new(
        request_id,
        CapabilityId::parse("clipboard.write").unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("set").unwrap(),
        retry_class,
        body.to_vec(),
    )
}

fn event(event_id: EventId, body: &[u8]) -> Event {
    Event::capability(
        event_id,
        CapabilityId::parse("clipboard.write").unwrap(),
        EventType::parse("clipboard.changed").unwrap(),
        body.to_vec(),
    )
    .unwrap()
}

fn system_event(event_id: EventId) -> Event {
    Event::system(
        event_id,
        EventType::parse("crosslab.system.keepalive").unwrap(),
        Vec::new(),
    )
    .unwrap()
}

fn inject(
    endpoint: &impl TransportConnection,
    protocol_version: ProtocolVersion,
    session_id: crosslab_policy::SessionId,
    sequence: u64,
    body: EnvelopeBody,
) {
    let envelope = ControlEnvelope::new(protocol_version, session_id, sequence, body);
    let frame = encode_control_envelope(&envelope).unwrap();
    endpoint.try_send_control(frame).unwrap();
}

#[test]
fn s006_authorized_request_response_and_event_are_correlated_and_sequenced() {
    let fixture = Fixture::new();
    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, true);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let local_caps = Fixture::local_capabilities();
    let mut node_a = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        local_caps.clone(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        fixture.allow_write_policy(),
        local_caps,
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();

    let request_id = RequestId::from_bytes([0x50; 16]);
    node_a
        .send_request(request(request_id, RetryClass::NonRetryable, b"hello"))
        .unwrap();
    assert_eq!(node_a.next_send_sequence(), Some(1));
    assert_eq!(node_a.pending_request_count(), 1);

    let NodeEvent::RequestDispatched(received) = node_b.receive_one().unwrap() else {
        panic!("expected authorized request dispatch");
    };
    assert_eq!(received.request_id(), request_id);
    assert_eq!(received.body(), b"hello");
    assert_eq!(node_b.expected_receive_sequence(), Some(1));

    node_b
        .send_response(request_id, ControlResponseResult::Success(b"ok".to_vec()))
        .unwrap();
    let NodeEvent::Response(response) = node_a.receive_one().unwrap() else {
        panic!("expected correlated response");
    };
    assert_eq!(response.request_id(), request_id);
    assert_eq!(node_a.pending_request_count(), 0);
    assert_eq!(node_a.expected_receive_sequence(), Some(1));

    let event_id = EventId::from_bytes([0x51; 16]);
    node_a.send_event(event(event_id, b"changed")).unwrap();
    let NodeEvent::Event(received) = node_b.receive_one().unwrap() else {
        panic!("expected permitted event");
    };
    assert_eq!(received.event_id(), event_id);
    assert_eq!(received.body(), b"changed");
    assert_eq!(node_a.next_send_sequence(), Some(2));
    assert_eq!(node_b.expected_receive_sequence(), Some(2));
}

#[test]
fn capability_advertisement_dispatch_updates_session_only_after_authentication() {
    let fixture = Fixture::new();
    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, false);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let local_caps = Fixture::local_capabilities();
    let mut node_a = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        local_caps.clone(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        local_caps,
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();

    assert!(
        node_b
            .session()
            .context()
            .unwrap()
            .negotiated_capabilities()
            .is_empty()
    );
    node_a
        .send_capability_advertisement(Fixture::advertisement())
        .unwrap();
    assert!(matches!(
        node_b.receive_one().unwrap(),
        NodeEvent::CapabilitiesUpdated
    ));
    let negotiated = node_b
        .session()
        .context()
        .unwrap()
        .negotiated_capabilities();
    assert_eq!(negotiated.len(), 1);
    assert_eq!(negotiated[0].capability_id().as_str(), "clipboard.write");
    assert_eq!(negotiated[0].version(), CapabilityVersion::new(1, 0));
}

#[test]
fn duplicate_or_gap_sequence_is_rejected_and_closes_the_session() {
    let fixture = Fixture::new();

    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, true);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let context = session_a.context().unwrap();
    inject(
        endpoint_a,
        context.protocol_version(),
        context.session_id(),
        0,
        EnvelopeBody::Event(system_event(EventId::from_bytes([0x60; 16]))),
    );
    inject(
        endpoint_a,
        context.protocol_version(),
        context.session_id(),
        0,
        EnvelopeBody::Event(system_event(EventId::from_bytes([0x61; 16]))),
    );
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    assert!(matches!(node_b.receive_one().unwrap(), NodeEvent::Event(_)));
    assert!(matches!(
        node_b.receive_one(),
        Err(NodeError::Dispatch(ControlDispatchError::Sequence(
            SequenceError::ReplayDetected {
                expected: 1,
                received: 0
            }
        )))
    ));
    assert_eq!(node_b.session().state(), SessionState::Closed);

    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, true);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let context = session_a.context().unwrap();
    inject(
        endpoint_a,
        context.protocol_version(),
        context.session_id(),
        1,
        EnvelopeBody::Event(system_event(EventId::from_bytes([0x62; 16]))),
    );
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        node_b.receive_one(),
        Err(NodeError::Dispatch(ControlDispatchError::Sequence(
            SequenceError::Gap {
                expected: 0,
                received: 1
            }
        )))
    ));
    assert_eq!(node_b.session().state(), SessionState::Closed);
}

#[test]
fn duplicate_nonretryable_request_id_is_rejected_after_first_completion() {
    let fixture = Fixture::new();
    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, true);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let context = session_a.context().unwrap();
    let request_id = RequestId::from_bytes([0x70; 16]);
    let request = request(request_id, RetryClass::NonRetryable, b"once");
    inject(
        endpoint_a,
        context.protocol_version(),
        context.session_id(),
        0,
        EnvelopeBody::ControlRequest(request.clone()),
    );
    inject(
        endpoint_a,
        context.protocol_version(),
        context.session_id(),
        1,
        EnvelopeBody::ControlRequest(request),
    );
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        fixture.allow_write_policy(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        node_b.receive_one().unwrap(),
        NodeEvent::RequestDispatched(_)
    ));
    node_b
        .send_response(request_id, ControlResponseResult::Success(Vec::new()))
        .unwrap();
    assert!(matches!(
        node_b.receive_one(),
        Err(NodeError::Dispatch(ControlDispatchError::DuplicateRequest))
    ));
}

#[test]
fn unauthorized_request_never_reaches_dispatch_and_session_remains_active() {
    let fixture = Fixture::new();
    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, true);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let mut node_a = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();

    node_a
        .send_request(request(
            RequestId::from_bytes([0x80; 16]),
            RetryClass::NonRetryable,
            b"denied",
        ))
        .unwrap();
    assert!(matches!(
        node_b.receive_one(),
        Err(NodeError::Dispatch(
            ControlDispatchError::AuthorizationDenied(DecisionReason::NoMatchingRule)
        ))
    ));
    assert_eq!(node_b.session().state(), SessionState::Active);
}

#[test]
fn cancellation_removes_pending_request_and_prevents_late_response() {
    let fixture = Fixture::new();
    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, true);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let mut node_a = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        fixture.allow_write_policy(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    let request_id = RequestId::from_bytes([0x90; 16]);

    node_a
        .send_request(request(request_id, RetryClass::Idempotent, b"work"))
        .unwrap();
    assert!(matches!(
        node_b.receive_one().unwrap(),
        NodeEvent::RequestDispatched(_)
    ));
    node_a.send_cancel(request_id).unwrap();
    assert_eq!(node_a.pending_request_count(), 0);
    assert!(matches!(
        node_b.receive_one().unwrap(),
        NodeEvent::RequestCancelled(id) if id == request_id
    ));
    assert!(matches!(
        node_b.send_response(request_id, ControlResponseResult::Success(Vec::new())),
        Err(NodeError::Dispatch(ControlDispatchError::UnknownRequest))
    ));
}

#[test]
fn malformed_frame_invalid_session_and_registered_close_fail_closed() {
    let fixture = Fixture::new();

    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, true);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    endpoint_a.try_send_control(vec![0, 0, 0, 1, 0xff]).unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    assert!(matches!(node_b.receive_one(), Err(NodeError::Wire(_))));
    assert_eq!(node_b.session().state(), SessionState::Closed);

    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, true);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let context = session_a.context().unwrap();
    inject(
        endpoint_a,
        context.protocol_version(),
        crosslab_policy::SessionId::from_bytes([0xee; 32]),
        0,
        EnvelopeBody::Event(system_event(EventId::from_bytes([0xa0; 16]))),
    );
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        node_b.receive_one(),
        Err(NodeError::Dispatch(ControlDispatchError::InvalidSession))
    ));
    assert_eq!(node_b.session().state(), SessionState::Closed);

    let pair = transport_pair();
    let (session_a, session_b) = fixture.sessions(&pair, true);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let mut node_a = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        Fixture::local_capabilities(),
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    node_a
        .send_close(SessionClose::new(SessionCloseReason::Normal, None))
        .unwrap();
    assert!(matches!(
        node_b.receive_one().unwrap(),
        NodeEvent::SessionClosed(SessionCloseReason::Normal)
    ));
    assert_eq!(node_b.session().state(), SessionState::Closed);
    assert!(endpoint_a.is_closed());
    assert!(endpoint_b.is_closed());
}
