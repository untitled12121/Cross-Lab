use std::num::NonZeroUsize;

use crosslab_core::{
    ChannelBinding, EventSubscription, LogicalSession, PairingFlowError, PairingId, PairingInstant,
    PairingInvitation, PairingInvitationState, PairingInviterFlow, PairingJoinerFlow,
    PairingSecret, SessionActivation, SessionAuthProof, SessionAuthRole, SessionAuthTranscriptV1,
    SessionError, SessionHandshakeSide, SessionState, TransportConnection, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, LocalCapability, NetworkClass,
    OperationName, PairingTrustTransition, PolicyRule, PolicyState, RuleEffect, RuleId,
    TransitionId, TrustRecord,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlRequest, ControlResponseResult,
    Event, EventId, EventType, FeatureSet, PairingHello, PairingRole, ProtocolRange,
    ProtocolVersion, RequestId, RetryClass,
};
use crosslab_sim::{
    node::{NodeEvent, SimNode},
    transport::MemoryTransportPair,
};

const CONTROL_CAPACITY: usize = 16;
const STATE_CAPACITY: usize = 16;
const JOINER_EPOCH: u64 = 0;
const INITIATOR_NONCE: [u8; 32] = [0x41; 32];
const RESPONDER_NONCE: [u8; 32] = [0x42; 32];

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

struct M5Fixture {
    owner_id: OwnerId,
    authority: OwnerAuthorityState,
    issuer_key: SigningKey,
    inviter_key: SigningKey,
    joiner_key: SigningKey,
    inviter_device_id: DeviceId,
    joiner_device_id: DeviceId,
    inviter_credential: DeviceCredential,
    pairing_id: PairingId,
    pairing_secret: [u8; 32],
    inviter_hello: PairingHello,
    joiner_hello: PairingHello,
}

impl M5Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x10; 32]);
        let root_key = SigningKey::from_secret_bytes([0x11; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x12; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();
        let inviter_key = SigningKey::from_secret_bytes([0x13; 32]);
        let joiner_key = SigningKey::from_secret_bytes([0x14; 32]);
        let inviter_device_id = DeviceId::from_bytes([0x15; 32]);
        let joiner_device_id = DeviceId::from_bytes([0x16; 32]);
        let inviter_credential = DeviceCredential::issue_current(
            owner_id,
            inviter_device_id,
            &inviter_key,
            3,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let pairing_id = PairingId::from_bytes([0x17; 16]);
        let pairing_secret = [0x18; 32];
        let inviter_hello = PairingHello::new(
            PairingRole::Inviter,
            1,
            pairing_id.to_bytes(),
            owner_id,
            inviter_device_id,
            inviter_key.verifying_key(),
            [0x19; 32],
        );
        let joiner_hello = PairingHello::new(
            PairingRole::Joiner,
            1,
            pairing_id.to_bytes(),
            owner_id,
            joiner_device_id,
            joiner_key.verifying_key(),
            [0x1a; 32],
        );

        Self {
            owner_id,
            authority,
            issuer_key,
            inviter_key,
            joiner_key,
            inviter_device_id,
            joiner_device_id,
            inviter_credential,
            pairing_id,
            pairing_secret,
            inviter_hello,
            joiner_hello,
        }
    }

    fn invitation(&self) -> PairingInvitation {
        PairingInvitation::from_parts(
            self.pairing_id,
            PairingSecret::from_bytes(self.pairing_secret),
            self.owner_id,
            self.inviter_device_id,
            PairingInstant::from_ticks(0),
            PairingInstant::from_ticks(100),
        )
        .unwrap()
    }

    fn complete_pairing(&self, credential_epoch: u64) -> (DeviceCredential, TrustRecord) {
        assert_eq!(credential_epoch, JOINER_EPOCH);
        let mut inviter = PairingInviterFlow::new(
            self.invitation(),
            self.inviter_hello,
            self.joiner_hello,
            PairingInstant::from_ticks(0),
        )
        .unwrap();
        let mut joiner = PairingJoinerFlow::new(
            PairingSecret::from_bytes(self.pairing_secret),
            self.inviter_hello,
            self.joiner_hello,
        )
        .unwrap();

        let joiner_confirmation = joiner.joiner_confirmation().unwrap();
        let inviter_confirmation = inviter
            .verify_joiner_confirmation(&joiner_confirmation, PairingInstant::from_ticks(10))
            .unwrap();
        joiner
            .verify_inviter_confirmation(&inviter_confirmation)
            .unwrap();

        let credential = inviter
            .issue_initial_joiner_credential(
                &self.authority,
                &self.issuer_key,
                PairingInstant::from_ticks(20),
            )
            .unwrap();
        let accepted = joiner
            .accept_credential(&self.authority, &credential, &self.joiner_key)
            .unwrap();
        let trust = inviter
            .commit_trust(
                &accepted,
                TransitionId::from_bytes([0x1b; 32]),
                &self.authority,
                &self.issuer_key,
                PairingInstant::from_ticks(30),
            )
            .unwrap();

        (credential, trust)
    }

    fn inviter_trust(&self) -> TrustRecord {
        establish_trust(
            &self.inviter_credential,
            &self.authority,
            &self.issuer_key,
            0x1c,
        )
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

    fn allow_write_policy(&self) -> PolicyState {
        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([0x1d; 32]),
                self.inviter_device_id,
                CapabilityId::parse("clipboard.write").unwrap(),
                OperationName::parse("set").unwrap(),
                RuleEffect::Allow,
            ))
            .unwrap();
        policy
    }

    fn proofs(
        &self,
        joiner_credential: &DeviceCredential,
        binding: &ChannelBinding,
        initiator_nonce: [u8; 32],
        responder_nonce: [u8; 32],
    ) -> (SessionAuthProof, SessionAuthProof) {
        let transcript = SessionAuthTranscriptV1::new(
            self.owner_id,
            &self.inviter_credential,
            initiator_nonce,
            joiner_credential,
            responder_nonce,
            ProtocolVersion::new(1, 0),
            &[1],
            binding.profile_id().as_bytes(),
            binding.bytes(),
        )
        .unwrap();
        let initiator_proof = transcript
            .create_proof(SessionAuthRole::Initiator, &self.inviter_key)
            .unwrap();
        let responder_proof = transcript
            .create_proof(SessionAuthRole::Responder, &self.joiner_key)
            .unwrap();
        (initiator_proof, responder_proof)
    }

    #[allow(clippy::too_many_arguments)]
    fn attempt_session(
        &self,
        joiner_credential: &DeviceCredential,
        local_role: SessionAuthRole,
        peer_trust: &TrustRecord,
        binding: &ChannelBinding,
        initiator_nonce: [u8; 32],
        responder_nonce: [u8; 32],
        initiator_proof: &SessionAuthProof,
        responder_proof: &SessionAuthProof,
    ) -> (LogicalSession, Result<(), SessionError>) {
        let ranges = Self::protocol_ranges();
        let features = Self::features();
        let initiator = SessionHandshakeSide::new(&self.inviter_credential, &ranges, &features);
        let responder = SessionHandshakeSide::new(joiner_credential, &ranges, &features);
        let mut session = LogicalSession::new();
        let result = session.authenticate(SessionActivation::new(
            &self.authority,
            initiator,
            responder,
            local_role,
            peer_trust,
            initiator_nonce,
            responder_nonce,
            binding,
            TransportSecurityClass::InProcessTest,
            initiator_proof,
            responder_proof,
        ));
        (session, result)
    }
}

fn transport_pair(binding: [u8; 32]) -> MemoryTransportPair {
    MemoryTransportPair::new(NonZeroUsize::new(CONTROL_CAPACITY).unwrap(), binding)
}

fn request(request_id: RequestId, body: &[u8]) -> ControlRequest {
    ControlRequest::new(
        request_id,
        CapabilityId::parse("clipboard.write").unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("set").unwrap(),
        RetryClass::NonRetryable,
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

#[test]
fn m5_end_to_end_pairing_session_capability_and_control_flow() {
    let fixture = M5Fixture::new();
    let (joiner_credential, joiner_trust) = fixture.complete_pairing(JOINER_EPOCH);
    let inviter_trust = fixture.inviter_trust();
    let pair = transport_pair([0x40; 32]);
    let (endpoint_a, endpoint_b) = pair.endpoints();
    let binding = endpoint_a.channel_binding();
    let (initiator_proof, responder_proof) = fixture.proofs(
        &joiner_credential,
        binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
    );

    let (session_a, result_a) = fixture.attempt_session(
        &joiner_credential,
        SessionAuthRole::Initiator,
        &joiner_trust,
        binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
        &initiator_proof,
        &responder_proof,
    );
    result_a.unwrap();
    let (session_b, result_b) = fixture.attempt_session(
        &joiner_credential,
        SessionAuthRole::Responder,
        &inviter_trust,
        binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
        &initiator_proof,
        &responder_proof,
    );
    result_b.unwrap();

    assert_eq!(session_a.state(), SessionState::Active);
    assert_eq!(session_b.state(), SessionState::Active);
    assert_eq!(
        session_a.context().unwrap().session_id(),
        session_b.context().unwrap().session_id()
    );

    let local_caps = M5Fixture::local_capabilities();
    let mut node_a = SimNode::new(
        session_a,
        endpoint_a,
        PolicyState::new(),
        local_caps.clone(),
        NetworkClass::Local,
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();
    let mut node_b = SimNode::new(
        session_b,
        endpoint_b,
        fixture.allow_write_policy(),
        local_caps,
        NetworkClass::Local,
        NonZeroUsize::new(STATE_CAPACITY).unwrap(),
    )
    .unwrap();

    node_a
        .send_capability_advertisement(M5Fixture::advertisement())
        .unwrap();
    assert!(matches!(
        node_b.receive_one(&inviter_trust).unwrap(),
        NodeEvent::CapabilitiesUpdated
    ));
    node_b
        .send_capability_advertisement(M5Fixture::advertisement())
        .unwrap();
    assert!(matches!(
        node_a.receive_one(&joiner_trust).unwrap(),
        NodeEvent::CapabilitiesUpdated
    ));

    let request_id = RequestId::from_bytes([0x50; 16]);
    node_a.send_request(request(request_id, b"hello")).unwrap();
    let NodeEvent::RequestDispatched(received) = node_b.receive_one(&inviter_trust).unwrap() else {
        panic!("expected authorized request dispatch");
    };
    assert_eq!(received.request_id(), request_id);
    assert_eq!(received.body(), b"hello");

    node_b
        .send_response(request_id, ControlResponseResult::Success(b"ok".to_vec()))
        .unwrap();
    let NodeEvent::Response(response) = node_a.receive_one(&joiner_trust).unwrap() else {
        panic!("expected correlated response");
    };
    assert_eq!(response.request_id(), request_id);

    assert!(
        node_b
            .subscribe_event(EventSubscription::new(
                CapabilityId::parse("clipboard.write").unwrap(),
                EventType::parse("clipboard.changed").unwrap(),
            ))
            .unwrap()
    );
    let event_id = EventId::from_bytes([0x51; 16]);
    node_a.send_event(event(event_id, b"changed")).unwrap();
    let NodeEvent::Event(received) = node_b.receive_one(&inviter_trust).unwrap() else {
        panic!("expected subscribed capability event");
    };
    assert_eq!(received.event_id(), event_id);
    assert_eq!(received.body(), b"changed");

    assert_eq!(node_a.next_send_sequence(), Some(3));
    assert_eq!(node_a.expected_receive_sequence(), Some(2));
    assert_eq!(node_b.next_send_sequence(), Some(2));
    assert_eq!(node_b.expected_receive_sequence(), Some(3));
}

#[test]
fn wrong_pairing_secret_fails_before_trust_commit() {
    let fixture = M5Fixture::new();
    let mut inviter = PairingInviterFlow::new(
        fixture.invitation(),
        fixture.inviter_hello,
        fixture.joiner_hello,
        PairingInstant::from_ticks(0),
    )
    .unwrap();
    let wrong_joiner = PairingJoinerFlow::new(
        PairingSecret::from_bytes([0xee; 32]),
        fixture.inviter_hello,
        fixture.joiner_hello,
    )
    .unwrap();
    let confirmation = wrong_joiner.joiner_confirmation().unwrap();

    assert_eq!(
        inviter.verify_joiner_confirmation(&confirmation, PairingInstant::from_ticks(10)),
        Err(PairingFlowError::InvalidConfirmation)
    );
    assert_eq!(inviter.invitation_state(), PairingInvitationState::Consumed);
}

#[test]
fn old_session_proof_under_fresh_nonce_is_rejected_closed() {
    let fixture = M5Fixture::new();
    let (joiner_credential, joiner_trust) = fixture.complete_pairing(JOINER_EPOCH);
    let pair = transport_pair([0x60; 32]);
    let (endpoint_a, _) = pair.endpoints();
    let binding = endpoint_a.channel_binding();
    let (initiator_proof, responder_proof) = fixture.proofs(
        &joiner_credential,
        binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
    );

    let (session, result) = fixture.attempt_session(
        &joiner_credential,
        SessionAuthRole::Initiator,
        &joiner_trust,
        binding,
        [0x61; 32],
        RESPONDER_NONCE,
        &initiator_proof,
        &responder_proof,
    );

    assert_eq!(
        result,
        Err(SessionError::Auth(
            crosslab_core::SessionAuthError::WrongProofTranscript
        ))
    );
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn session_proofs_bound_to_another_channel_are_rejected_closed() {
    let fixture = M5Fixture::new();
    let (joiner_credential, joiner_trust) = fixture.complete_pairing(JOINER_EPOCH);
    let pair = transport_pair([0x70; 32]);
    let (endpoint_a, _) = pair.endpoints();
    let proof_binding = endpoint_a.channel_binding();
    let (initiator_proof, responder_proof) = fixture.proofs(
        &joiner_credential,
        proof_binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
    );
    let wrong_binding = ChannelBinding::new("in-process-test", vec![0x71; 32]);

    let (session, result) = fixture.attempt_session(
        &joiner_credential,
        SessionAuthRole::Initiator,
        &joiner_trust,
        &wrong_binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
        &initiator_proof,
        &responder_proof,
    );

    assert_eq!(
        result,
        Err(SessionError::Auth(
            crosslab_core::SessionAuthError::WrongProofTranscript
        ))
    );
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn stale_accepted_credential_epoch_is_rejected_before_session_activation() {
    let fixture = M5Fixture::new();
    let (joiner_credential, _) = fixture.complete_pairing(JOINER_EPOCH);
    let newer_credential = DeviceCredential::issue_for_public_key_current(
        fixture.owner_id,
        fixture.joiner_device_id,
        fixture.joiner_key.verifying_key(),
        JOINER_EPOCH + 1,
        &fixture.authority,
        &fixture.issuer_key,
    )
    .unwrap();
    let stale_trust = establish_trust(
        &newer_credential,
        &fixture.authority,
        &fixture.issuer_key,
        0x80,
    );
    let pair = transport_pair([0x81; 32]);
    let (endpoint_a, _) = pair.endpoints();
    let binding = endpoint_a.channel_binding();
    let (initiator_proof, responder_proof) = fixture.proofs(
        &joiner_credential,
        binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
    );

    let (session, result) = fixture.attempt_session(
        &joiner_credential,
        SessionAuthRole::Initiator,
        &stale_trust,
        binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
        &initiator_proof,
        &responder_proof,
    );

    assert_eq!(result, Err(SessionError::PeerCredentialEpochMismatch));
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn owner_mismatch_in_peer_trust_is_rejected_before_session_activation() {
    let fixture = M5Fixture::new();
    let (joiner_credential, _) = fixture.complete_pairing(JOINER_EPOCH);
    let wrong_owner = OwnerId::from_bytes([0x90; 32]);
    let wrong_root_key = SigningKey::from_secret_bytes([0x93; 32]);
    let wrong_root = OwnerRootRecord::new(wrong_owner, &wrong_root_key, 0);
    let wrong_issuer_key = SigningKey::from_secret_bytes([0x94; 32]);
    let wrong_delegation = AuthorityDelegation::issue(
        wrong_owner,
        AuthorityRole::DeviceSigning,
        &wrong_issuer_key,
        0,
        &wrong_root_key,
    );
    let mut wrong_authority = OwnerAuthorityState::new(wrong_root);
    wrong_authority.accept_delegation(wrong_delegation).unwrap();
    let wrong_credential = DeviceCredential::issue_for_public_key_current(
        wrong_owner,
        fixture.joiner_device_id,
        fixture.joiner_key.verifying_key(),
        JOINER_EPOCH,
        &wrong_authority,
        &wrong_issuer_key,
    )
    .unwrap();
    let wrong_owner_trust =
        establish_trust(&wrong_credential, &wrong_authority, &wrong_issuer_key, 0x91);
    let pair = transport_pair([0x92; 32]);
    let (endpoint_a, _) = pair.endpoints();
    let binding = endpoint_a.channel_binding();
    let (initiator_proof, responder_proof) = fixture.proofs(
        &joiner_credential,
        binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
    );

    let (session, result) = fixture.attempt_session(
        &joiner_credential,
        SessionAuthRole::Initiator,
        &wrong_owner_trust,
        binding,
        INITIATOR_NONCE,
        RESPONDER_NONCE,
        &initiator_proof,
        &responder_proof,
    );

    assert_eq!(result, Err(SessionError::PeerTrustMismatch));
    assert_eq!(session.state(), SessionState::Closed);
}
