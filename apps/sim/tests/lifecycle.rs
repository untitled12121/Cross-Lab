use std::num::NonZeroUsize;

use crosslab_core::{
    ControlDispatchError, LogicalSession, SessionActivation, SessionAuthRole,
    SessionAuthTranscriptV1, SessionHandshakeSide, SessionState, StreamAdmission,
    StreamAdmissionError, TransportConnection, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    AuthorizationContext, AuthorizedOperation, CapabilityId, CapabilityVersion,
    CapabilityVersionRange, LocalCapability, NetworkClass, OperationError, OperationName,
    PolicyRule, PolicyState, RuleEffect, RuleId, TransitionId, TrustRecord, TrustState, UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlEnvelope, ControlRequest,
    DataStreamOpen, EnvelopeBody, Event, EventId, EventType, FeatureSet, ProtocolRange,
    ProtocolVersion, RequestId, RetryClass, StreamDirection, StreamId, encode_control_envelope,
};
use crosslab_sim::{
    node::{NodeError, NodeEvent, SimNode},
    transport::MemoryTransportPair,
};

const CAPACITY: usize = 8;

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
        let owner_id = OwnerId::from_bytes([0xd0; 32]);
        let root_key = SigningKey::from_secret_bytes([0xd1; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0xd2; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let initiator_key = SigningKey::from_secret_bytes([0xd3; 32]);
        let responder_key = SigningKey::from_secret_bytes([0xd4; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0xd5; 32]),
            &initiator_key,
            1,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0xd6; 32]),
            &responder_key,
            1,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let initiator_trust = TrustRecord::trusted(
            owner_id,
            initiator_credential.device_id(),
            initiator_credential.credential_epoch(),
            TransitionId::from_bytes([0xd7; 32]),
        );
        let responder_trust = TrustRecord::trusted(
            owner_id,
            responder_credential.device_id(),
            responder_credential.credential_epoch(),
            TransitionId::from_bytes([0xd8; 32]),
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

    fn sessions(
        &self,
        pair: &MemoryTransportPair,
        initiator_nonce: [u8; 32],
        responder_nonce: [u8; 32],
    ) -> (LogicalSession, LogicalSession) {
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = pair.endpoints().0.channel_binding();
        let transcript = SessionAuthTranscriptV1::new(
            self.owner_id,
            &self.initiator_credential,
            initiator_nonce,
            &self.responder_credential,
            responder_nonce,
            ProtocolVersion::new(1, 0),
            &[],
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
                initiator_nonce,
                responder_nonce,
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
                initiator_nonce,
                responder_nonce,
                binding,
                TransportSecurityClass::InProcessTest,
                &initiator_proof,
                &responder_proof,
            ))
            .unwrap();

        (session_a, session_b)
    }

    fn local_capability() -> LocalCapability {
        LocalCapability::new(
            CapabilityId::parse("files.transfer").unwrap(),
            CapabilityVersionRange::new(1, 0, 0).unwrap(),
            true,
        )
    }

    fn advertisement() -> CapabilityAdvertisement {
        CapabilityAdvertisement::new(vec![
            CapabilityAdvertisementEntry::new(
                CapabilityId::parse("files.transfer").unwrap(),
                CapabilityVersion::new(1, 0),
                CapabilityVersion::new(1, 0),
                true,
            )
            .unwrap(),
        ])
        .unwrap()
    }

    fn old_operation(
        &self,
        session: &LogicalSession,
    ) -> (AuthorizedOperation, u64, u64) {
        let capability = CapabilityId::parse("files.transfer").unwrap();
        let version = CapabilityVersion::new(1, 0);
        let operation_name = OperationName::parse("send").unwrap();
        let local_capability = Self::local_capability();
        let context = session.context().unwrap();
        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([0xd9; 32]),
                self.initiator_credential.device_id(),
                capability.clone(),
                operation_name.clone(),
                RuleEffect::Allow,
            ))
            .unwrap();
        let authorization = AuthorizationContext::new(
            self.initiator_credential.device_id(),
            self.responder_credential.device_id(),
            context.session_id(),
            capability,
            version,
            operation_name,
            TrustState::Trusted,
            self.initiator_trust.trust_revision(),
            local_capability,
            NetworkClass::Local,
        );
        let grant = policy.evaluate(&authorization).into_grant().unwrap();
        (
            AuthorizedOperation::issue(grant, 10, 20, UsePolicy::SingleStream).unwrap(),
            self.initiator_trust.trust_revision(),
            policy.revision(),
        )
    }
}

fn pair(binding_byte: u8) -> MemoryTransportPair {
    MemoryTransportPair::new(
        NonZeroUsize::new(CAPACITY).unwrap(),
        [binding_byte; 32],
    )
}

fn request(byte: u8) -> ControlRequest {
    ControlRequest::new(
        RequestId::from_bytes([byte; 16]),
        CapabilityId::parse("files.transfer").unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("send").unwrap(),
        RetryClass::NonRetryable,
        vec![byte],
    )
}

fn exchange_capabilities(node_a: &mut SimNode<'_>, node_b: &mut SimNode<'_>) {
    node_a
        .send_capability_advertisement(Fixture::advertisement())
        .unwrap();
    assert_eq!(node_b.receive_one().unwrap(), NodeEvent::CapabilitiesUpdated);
    node_b
        .send_capability_advertisement(Fixture::advertisement())
        .unwrap();
    assert_eq!(node_a.receive_one().unwrap(), NodeEvent::CapabilitiesUpdated);
}

#[test]
fn s008_disconnect_reconnect_creates_fresh_session_and_capability_state() {
    let fixture = Fixture::new();
    let old_pair = pair(0xe0);
    let (old_a, old_b) = fixture.sessions(&old_pair, [0xe1; 32], [0xe2; 32]);
    let old_session_id = old_a.context().unwrap().session_id();
    let (old_endpoint_a, old_endpoint_b) = old_pair.endpoints();
    let local_capability = Fixture::local_capability();
    let mut old_node_a = SimNode::new(
        old_a,
        old_endpoint_a,
        PolicyState::new(),
        vec![local_capability.clone()],
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();
    let mut old_node_b = SimNode::new(
        old_b,
        old_endpoint_b,
        PolicyState::new(),
        vec![local_capability.clone()],
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    exchange_capabilities(&mut old_node_a, &mut old_node_b);
    assert_eq!(
        old_node_a
            .session()
            .context()
            .unwrap()
            .negotiated_capabilities()
            .len(),
        1
    );
    old_node_a.send_request(request(0xe3)).unwrap();
    assert_eq!(old_node_a.pending_request_count(), 1);

    old_pair.faults().disconnect_now();
    assert!(matches!(
        old_node_a.receive_one(),
        Err(NodeError::Receive(crosslab_core::ControlReceiveError::Closed))
    ));
    assert_eq!(old_node_a.session().state(), SessionState::Closed);
    assert_eq!(old_node_a.pending_request_count(), 0);

    let new_pair = pair(0xe4);
    let (new_a, new_b) = fixture.sessions(&new_pair, [0xe5; 32], [0xe6; 32]);
    let new_session_id = new_a.context().unwrap().session_id();
    assert_ne!(new_session_id, old_session_id);
    assert_eq!(new_a.context().unwrap().next_send_sequence(), 0);
    assert_eq!(new_a.context().unwrap().next_receive_sequence(), 0);
    assert!(new_a.context().unwrap().negotiated_capabilities().is_empty());
    assert!(new_b.context().unwrap().negotiated_capabilities().is_empty());

    let (new_endpoint_a, new_endpoint_b) = new_pair.endpoints();
    let mut new_node_a = SimNode::new(
        new_a,
        new_endpoint_a,
        PolicyState::new(),
        vec![local_capability.clone()],
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();
    let mut new_node_b = SimNode::new(
        new_b,
        new_endpoint_b,
        PolicyState::new(),
        vec![local_capability],
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    assert_eq!(new_node_a.next_send_sequence(), Some(0));
    assert_eq!(new_node_a.expected_receive_sequence(), Some(0));
    exchange_capabilities(&mut new_node_a, &mut new_node_b);
    assert_eq!(
        new_node_a
            .session()
            .context()
            .unwrap()
            .negotiated_capabilities()
            .len(),
        1
    );
    assert_eq!(
        new_node_b
            .session()
            .context()
            .unwrap()
            .negotiated_capabilities()
            .len(),
        1
    );
}

#[test]
fn s008_old_control_envelope_is_rejected_by_new_session() {
    let fixture = Fixture::new();
    let old_pair = pair(0xe7);
    let (old_a, _) = fixture.sessions(&old_pair, [0xe8; 32], [0xe9; 32]);
    let old_session_id = old_a.context().unwrap().session_id();

    let new_pair = pair(0xea);
    let (_, new_b) = fixture.sessions(&new_pair, [0xeb; 32], [0xec; 32]);
    let (new_endpoint_a, new_endpoint_b) = new_pair.endpoints();
    let mut new_node_b = SimNode::new(
        new_b,
        new_endpoint_b,
        PolicyState::new(),
        vec![Fixture::local_capability()],
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    let event = Event::system(
        EventId::from_bytes([0xed; 16]),
        EventType::parse("crosslab.system.keepalive").unwrap(),
        Vec::new(),
    )
    .unwrap();
    let envelope = ControlEnvelope::new(
        ProtocolVersion::new(1, 0),
        old_session_id,
        0,
        EnvelopeBody::Event(event),
    );
    new_endpoint_a
        .try_send_control(encode_control_envelope(&envelope).unwrap())
        .unwrap();

    assert!(matches!(
        new_node_b.receive_one(),
        Err(NodeError::Dispatch(ControlDispatchError::InvalidSession))
    ));
    assert_eq!(new_node_b.session().state(), SessionState::Closed);
    assert!(new_endpoint_b.is_closed());
}

#[test]
fn s008_old_operation_cannot_authorize_reconnected_session() {
    let fixture = Fixture::new();
    let old_pair = pair(0xee);
    let (_, old_b) = fixture.sessions(&old_pair, [0xef; 32], [0xf0; 32]);
    let (old_operation, trust_revision, policy_revision) = fixture.old_operation(&old_b);
    let old_operation_id = old_operation.id();

    let new_pair = pair(0xf1);
    let (_, mut new_b) = fixture.sessions(&new_pair, [0xf2; 32], [0xf3; 32]);
    new_b
        .negotiate_capabilities(
            &[Fixture::local_capability()],
            &Fixture::advertisement(),
        )
        .unwrap();
    let new_session_id = new_b.context().unwrap().session_id();
    assert_ne!(new_session_id, old_b.context().unwrap().session_id());

    let mut admission = StreamAdmission::new(NonZeroUsize::new(CAPACITY).unwrap());
    admission.register_operation(old_operation).unwrap();
    let open = DataStreamOpen::new(
        new_session_id,
        StreamId::from_bytes([0xf4; 16]),
        old_operation_id,
        CapabilityId::parse("files.transfer").unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("send").unwrap(),
        StreamDirection::SourceToDestination,
        0,
    );

    assert_eq!(
        admission.admit_inbound(
            &new_b,
            &open,
            15,
            trust_revision,
            policy_revision,
        ),
        Err(StreamAdmissionError::Operation(OperationError::BindingMismatch))
    );
}
