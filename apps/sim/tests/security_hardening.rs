use std::num::NonZeroUsize;

use crosslab_core::{
    ControlDispatchError, LogicalSession, SessionActivation, SessionAuthRole,
    SessionAuthTranscriptV1, SessionHandshakeSide, SessionState, TransportConnection,
    TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, Constraint, DecisionReason,
    LocalCapability, NetworkClass, OperationName, PairingTrustTransition, PolicyRule, PolicyState,
    RuleEffect, RuleId, TransitionId, TrustRecord, TrustTransition,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlRequest, FeatureSet,
    ProtocolRange, ProtocolVersion, RequestId, RetryClass,
};
use crosslab_sim::{
    node::{NodeError, SimNode},
    transport::MemoryTransportPair,
};

const CAPACITY: usize = 8;

fn establish_trust(
    credential: &DeviceCredential,
    authority: &OwnerAuthorityState,
    issuer_key: &SigningKey,
    transition_byte: u8,
) -> TrustRecord {
    let initial_credential = DeviceCredential::issue_for_public_key_current(
        credential.owner_id(),
        credential.device_id(),
        credential.device_public_key(),
        0,
        authority,
        issuer_key,
    )
    .unwrap();
    let transition = PairingTrustTransition::issue_current(
        &initial_credential,
        TransitionId::from_bytes([transition_byte; 32]),
        [transition_byte.wrapping_add(1); 32],
        authority,
        issuer_key,
    )
    .unwrap();
    let mut trust = transition
        .establish_current(&initial_credential, authority)
        .unwrap();

    for epoch in 1..=credential.credential_epoch() {
        let successor = DeviceCredential::issue_for_public_key_current(
            credential.owner_id(),
            credential.device_id(),
            credential.device_public_key(),
            epoch,
            authority,
            issuer_key,
        )
        .unwrap();
        let mut transition_id = [transition_byte; 32];
        transition_id[..8].copy_from_slice(&epoch.to_be_bytes());
        trust
            .accept_successor_credential_current(
                &successor,
                authority,
                TransitionId::from_bytes(transition_id),
            )
            .unwrap();
    }

    trust
}

struct Fixture {
    owner_id: OwnerId,
    root_key: SigningKey,
    authority: OwnerAuthorityState,
    issuer_key: SigningKey,
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
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();
        let initiator_key = SigningKey::from_secret_bytes([0xd3; 32]);
        let responder_key = SigningKey::from_secret_bytes([0xd4; 32]);
        let initiator_credential = DeviceCredential::issue_current(
            owner_id,
            DeviceId::from_bytes([0xd5; 32]),
            &initiator_key,
            1,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue_current(
            owner_id,
            DeviceId::from_bytes([0xd6; 32]),
            &responder_key,
            1,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let initiator_trust = establish_trust(&initiator_credential, &authority, &issuer_key, 0xd7);
        let responder_trust = establish_trust(&responder_credential, &authority, &issuer_key, 0xd8);

        Self {
            owner_id,
            root_key,
            authority,
            issuer_key,
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
            initiator_trust,
            responder_trust,
        }
    }

    fn capability() -> CapabilityId {
        CapabilityId::parse("files.transfer").unwrap()
    }

    fn operation() -> OperationName {
        OperationName::parse("send").unwrap()
    }

    fn local_capabilities() -> Vec<LocalCapability> {
        vec![LocalCapability::new(
            Self::capability(),
            CapabilityVersionRange::new(1, 0, 0).unwrap(),
            true,
        )]
    }

    fn advertisement() -> CapabilityAdvertisement {
        CapabilityAdvertisement::new(vec![
            CapabilityAdvertisementEntry::new(
                Self::capability(),
                CapabilityVersion::new(1, 0),
                CapabilityVersion::new(1, 0),
                true,
            )
            .unwrap(),
        ])
        .unwrap()
    }

    fn sessions(&self, pair: &MemoryTransportPair) -> (LogicalSession, LogicalSession) {
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = pair.endpoints().0.channel_binding();
        let initiator_nonce = [0xd9; 32];
        let responder_nonce = [0xda; 32];
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
        let initiator = SessionHandshakeSide::new(&self.initiator_credential, &ranges, &features);
        let responder = SessionHandshakeSide::new(&self.responder_credential, &ranges, &features);

        let mut session_a = LogicalSession::new();
        session_a
            .authenticate(SessionActivation::new(
                &self.authority,
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
                &self.authority,
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

        let local = Self::local_capabilities();
        let advertisement = Self::advertisement();
        session_a
            .negotiate_capabilities(&local, &advertisement)
            .unwrap();
        session_b
            .negotiate_capabilities(&local, &advertisement)
            .unwrap();
        (session_a, session_b)
    }

    fn policy(&self, local_only: bool) -> PolicyState {
        let mut rule = PolicyRule::new(
            RuleId::from_bytes([0xdb; 32]),
            self.initiator_credential.device_id(),
            Self::capability(),
            Self::operation(),
            RuleEffect::Allow,
        );
        if local_only {
            rule = rule.with_constraint(Constraint::LocalOnly);
        }
        let mut policy = PolicyState::new();
        policy.insert(rule).unwrap();
        policy
    }

    fn revoke_initiator(&self) -> TrustRecord {
        let transition = TrustTransition::issue_root_revocation(
            &self.initiator_trust,
            TransitionId::from_bytes([0xdc; 32]),
            self.authority.root(),
            &self.root_key,
        )
        .unwrap();
        let mut revoked = self.initiator_trust;
        transition
            .apply_root(&mut revoked, self.authority.root())
            .unwrap();
        revoked
    }

    fn wrong_device_trust(&self) -> TrustRecord {
        let credential = DeviceCredential::issue_current(
            self.owner_id,
            DeviceId::from_bytes([0xe0; 32]),
            &SigningKey::from_secret_bytes([0xe3; 32]),
            self.initiator_credential.credential_epoch(),
            &self.authority,
            &self.issuer_key,
        )
        .unwrap();
        establish_trust(&credential, &self.authority, &self.issuer_key, 0xe1)
    }
}

fn pair() -> MemoryTransportPair {
    MemoryTransportPair::new(NonZeroUsize::new(CAPACITY).unwrap(), [0xdd; 32])
}

fn request(byte: u8) -> ControlRequest {
    ControlRequest::new(
        RequestId::from_bytes([byte; 16]),
        Fixture::capability(),
        CapabilityVersion::new(1, 0),
        Fixture::operation(),
        RetryClass::NonRetryable,
        vec![byte],
    )
}

#[test]
fn remote_session_cannot_satisfy_local_only_policy() {
    let fixture = Fixture::new();
    let pair = pair();
    let (session_a, session_b) = fixture.sessions(&pair);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let local = Fixture::local_capabilities();
    let mut node_a = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        local.clone(),
        NetworkClass::Local,
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        fixture.policy(true),
        local,
        NetworkClass::Remote,
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    node_a.send_request(request(0xde)).unwrap();
    assert!(matches!(
        node_b.receive_one(&fixture.initiator_trust),
        Err(NodeError::Dispatch(
            ControlDispatchError::AuthorizationDenied(DecisionReason::ConstraintFailed)
        ))
    ));
    assert_eq!(node_b.session().state(), SessionState::Active);
}

#[test]
fn locally_revoked_peer_is_rejected_before_explicit_teardown() {
    let fixture = Fixture::new();
    let pair = pair();
    let (session_a, session_b) = fixture.sessions(&pair);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let local = Fixture::local_capabilities();
    let mut node_a = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        local.clone(),
        NetworkClass::Local,
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        fixture.policy(false),
        local,
        NetworkClass::Local,
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();

    node_a.send_request(request(0xdf)).unwrap();
    let revoked = fixture.revoke_initiator();
    assert!(matches!(
        node_b.receive_one(&revoked),
        Err(NodeError::Dispatch(
            ControlDispatchError::AuthorizationDenied(DecisionReason::RevokedPeer)
        ))
    ));
    assert_eq!(node_b.session().state(), SessionState::Closed);
    assert!(endpoint_b.is_closed());
}

#[test]
fn trust_record_for_another_device_is_fatal_provenance_mismatch() {
    let fixture = Fixture::new();
    let pair = pair();
    let (session_a, session_b) = fixture.sessions(&pair);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let local = Fixture::local_capabilities();
    let mut node_a = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        local.clone(),
        NetworkClass::Local,
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        fixture.policy(false),
        local,
        NetworkClass::Local,
        NonZeroUsize::new(CAPACITY).unwrap(),
    )
    .unwrap();
    let wrong_trust = fixture.wrong_device_trust();

    node_a.send_request(request(0xe2)).unwrap();
    assert!(matches!(
        node_b.receive_one(&wrong_trust),
        Err(NodeError::Dispatch(ControlDispatchError::PeerTrustMismatch))
    ));
    assert_eq!(node_b.session().state(), SessionState::Closed);
    assert!(endpoint_b.is_closed());
}
