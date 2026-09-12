use std::num::NonZeroUsize;

use crosslab_core::{
    ControlReceiveError, ControlSendError, LogicalSession, SessionActivation, SessionAuthRole,
    SessionAuthTranscriptV1, SessionError, SessionHandshakeSide, SessionState, TransportConnection,
    TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    CapabilityId, CapabilityVersion, OperationName, PolicyState, TransitionId, TrustRecord,
    TrustTransition,
};
use crosslab_protocol::{
    ControlRequest, FeatureSet, ProtocolRange, ProtocolVersion, RequestId, RetryClass,
};
use crosslab_sim::{
    node::{NodeError, SimNode},
    transport::MemoryTransportPair,
};

const CAPACITY: usize = 4;

struct Fixture {
    owner_id: OwnerId,
    root_key: SigningKey,
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
        let owner_id = OwnerId::from_bytes([0xb0; 32]);
        let root_key = SigningKey::from_secret_bytes([0xb1; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0xb2; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let initiator_key = SigningKey::from_secret_bytes([0xb3; 32]);
        let responder_key = SigningKey::from_secret_bytes([0xb4; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0xb5; 32]),
            &initiator_key,
            1,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0xb6; 32]),
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
            TransitionId::from_bytes([0xb7; 32]),
        );
        let responder_trust = TrustRecord::trusted(
            owner_id,
            responder_credential.device_id(),
            responder_credential.credential_epoch(),
            TransitionId::from_bytes([0xb8; 32]),
        );

        Self {
            owner_id,
            root_key,
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

    fn sessions(&self, pair: &MemoryTransportPair) -> (LogicalSession, LogicalSession) {
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = pair.endpoints().0.channel_binding();
        let initiator_nonce = [0xb9; 32];
        let responder_nonce = [0xba; 32];
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

        let mut a = LogicalSession::new();
        a.authenticate(SessionActivation::new(
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

        let mut b = LogicalSession::new();
        b.authenticate(SessionActivation::new(
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

        (a, b)
    }

    fn revoke(&self, record: TrustRecord, transition_byte: u8) -> TrustRecord {
        let transition = TrustTransition::issue_root_revocation(
            &record,
            TransitionId::from_bytes([transition_byte; 32]),
            &self.root,
            &self.root_key,
        )
        .unwrap();
        let mut revoked = record;
        transition.apply_root(&mut revoked, &self.root).unwrap();
        revoked
    }
}

fn pair() -> MemoryTransportPair {
    MemoryTransportPair::new(NonZeroUsize::new(CAPACITY).unwrap(), [0xbb; 32])
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

#[test]
fn send_side_transport_loss_closes_session_and_discards_pending_authority() {
    let fixture = Fixture::new();
    let pair = pair();
    let (session_a, _) = fixture.sessions(&pair);
    let (endpoint_a, _) = pair.endpoints();
    let mut node = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        Vec::new(),
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    node.send_request(request(0xc0)).unwrap();
    assert_eq!(node.pending_request_count(), 1);

    pair.faults().disconnect_now();
    assert!(matches!(
        node.send_request(request(0xc1)),
        Err(NodeError::Send(ControlSendError::Closed(_)))
    ));
    assert_eq!(node.session().state(), SessionState::Closed);
    assert_eq!(node.pending_request_count(), 0);
}

#[test]
fn receive_side_transport_loss_closes_active_session() {
    let fixture = Fixture::new();
    let pair = pair();
    let (_, session_b) = fixture.sessions(&pair);
    let (_, endpoint_b) = pair.endpoints();
    let mut node = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        Vec::new(),
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    pair.faults().disconnect_now();
    assert!(matches!(
        node.receive_one(),
        Err(NodeError::Receive(ControlReceiveError::Closed))
    ));
    assert_eq!(node.session().state(), SessionState::Closed);
}

#[test]
fn accepted_peer_revocation_closes_session_transport_and_pending_authority() {
    let fixture = Fixture::new();
    let revoked_initiator = fixture.revoke(fixture.initiator_trust, 0xc2);
    let pair = pair();
    let (_, session_b) = fixture.sessions(&pair);
    let (_, endpoint_b) = pair.endpoints();
    let mut node = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        Vec::new(),
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    node.send_request(request(0xc3)).unwrap();
    assert_eq!(node.pending_request_count(), 1);

    node.apply_peer_revocation(&revoked_initiator).unwrap();
    assert_eq!(node.session().state(), SessionState::Closed);
    assert_eq!(node.pending_request_count(), 0);
    assert!(endpoint_b.is_closed());
}

#[test]
fn wrong_peer_revocation_cannot_terminate_an_unrelated_session() {
    let fixture = Fixture::new();
    let wrong = TrustRecord::trusted(
        fixture.owner_id,
        DeviceId::from_bytes([0xc4; 32]),
        fixture.initiator_credential.credential_epoch(),
        TransitionId::from_bytes([0xc5; 32]),
    );
    let wrong = fixture.revoke(wrong, 0xc6);
    let pair = pair();
    let (_, session_b) = fixture.sessions(&pair);
    let (_, endpoint_b) = pair.endpoints();
    let mut node = SimNode::new(
        session_b,
        endpoint_b,
        PolicyState::new(),
        Vec::new(),
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        node.apply_peer_revocation(&wrong),
        Err(NodeError::Session(SessionError::PeerTrustMismatch))
    ));
    assert_eq!(node.session().state(), SessionState::Active);
    assert!(!endpoint_b.is_closed());
}

#[test]
fn shutdown_is_idempotent_and_discards_pending_authority() {
    let fixture = Fixture::new();
    let pair = pair();
    let (session_a, _) = fixture.sessions(&pair);
    let (endpoint_a, _) = pair.endpoints();
    let mut node = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        Vec::new(),
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    node.send_request(request(0xc7)).unwrap();
    assert_eq!(node.pending_request_count(), 1);

    node.shutdown();
    node.shutdown();
    assert_eq!(node.session().state(), SessionState::Closed);
    assert_eq!(node.pending_request_count(), 0);
    assert!(endpoint_a.is_closed());
}
