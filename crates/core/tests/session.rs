use crosslab_core::{
    ChannelBinding, LogicalSession, SessionActivation, SessionAuthError, SessionAuthProof,
    SessionAuthRole, SessionAuthTranscriptV1, SessionError, SessionHandshakeSide, SessionState,
    TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    AuthorizationContext, CapabilityId, CapabilityVersion, CapabilityVersionRange, DecisionEffect,
    DecisionReason, LocalCapability, NetworkClass, OperationName, PolicyState, TransitionId,
    TrustRecord, TrustState,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, FeatureNegotiationError, FeatureSet,
    ProtocolRange, ProtocolVersion, VersionNegotiationError, negotiate_features,
    negotiate_protocol_version,
};

struct Fixture {
    owner_id: OwnerId,
    root: OwnerRootRecord,
    delegation: AuthorityDelegation,
    initiator_key: SigningKey,
    responder_key: SigningKey,
    initiator_credential: DeviceCredential,
    responder_credential: DeviceCredential,
    responder_trust: TrustRecord,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x40; 32]);
        let root_key = SigningKey::from_secret_bytes([0x41; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x42; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let initiator_key = SigningKey::from_secret_bytes([0x43; 32]);
        let responder_key = SigningKey::from_secret_bytes([0x44; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x45; 32]),
            &initiator_key,
            2,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x46; 32]),
            &responder_key,
            5,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let responder_trust = TrustRecord::trusted(
            owner_id,
            responder_credential.device_id(),
            responder_credential.credential_epoch(),
            TransitionId::from_bytes([0x47; 32]),
        );

        Self {
            owner_id,
            root,
            delegation,
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
            responder_trust,
        }
    }

    fn local_ranges() -> [ProtocolRange; 1] {
        [ProtocolRange::new(1, 0, 3).unwrap()]
    }

    fn peer_ranges() -> [ProtocolRange; 1] {
        [ProtocolRange::new(1, 1, 2).unwrap()]
    }

    fn local_features() -> FeatureSet {
        FeatureSet::new(&[1, 2, 3], &[2]).unwrap()
    }

    fn peer_features() -> FeatureSet {
        FeatureSet::new(&[2, 3, 4], &[3]).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn transcript(
        &self,
        initiator_nonce: [u8; 32],
        responder_nonce: [u8; 32],
        channel_binding: &ChannelBinding,
        local_ranges: &[ProtocolRange],
        peer_ranges: &[ProtocolRange],
        local_features: &FeatureSet,
        peer_features: &FeatureSet,
    ) -> SessionAuthTranscriptV1 {
        let protocol = negotiate_protocol_version(local_ranges, peer_ranges).unwrap();
        let features = negotiate_features(local_features, peer_features).unwrap();
        SessionAuthTranscriptV1::new(
            self.owner_id,
            &self.initiator_credential,
            initiator_nonce,
            &self.responder_credential,
            responder_nonce,
            protocol,
            &features,
            channel_binding.profile_id().as_bytes(),
            channel_binding.bytes(),
        )
        .unwrap()
    }

    fn proofs(&self, transcript: &SessionAuthTranscriptV1) -> (SessionAuthProof, SessionAuthProof) {
        (
            transcript
                .create_proof(SessionAuthRole::Initiator, &self.initiator_key)
                .unwrap(),
            transcript
                .create_proof(SessionAuthRole::Responder, &self.responder_key)
                .unwrap(),
        )
    }
}

#[test]
fn valid_authentication_activates_with_fresh_context_and_zero_sequences() {
    let fixture = Fixture::new();
    let local_ranges = Fixture::local_ranges();
    let peer_ranges = Fixture::peer_ranges();
    let local_features = Fixture::local_features();
    let peer_features = Fixture::peer_features();
    let binding = ChannelBinding::new("in-process-test", vec![0x51; 32]);
    let transcript = fixture.transcript(
        [0x52; 32],
        [0x53; 32],
        &binding,
        &local_ranges,
        &peer_ranges,
        &local_features,
        &peer_features,
    );
    let (initiator_proof, responder_proof) = fixture.proofs(&transcript);
    let expected_session_id = transcript
        .derive_session_id(
            &initiator_proof,
            fixture.initiator_key.verifying_key(),
            &responder_proof,
            fixture.responder_key.verifying_key(),
        )
        .unwrap();
    let initiator = SessionHandshakeSide::new(
        &fixture.initiator_credential,
        &fixture.delegation,
        &local_ranges,
        &local_features,
    );
    let responder = SessionHandshakeSide::new(
        &fixture.responder_credential,
        &fixture.delegation,
        &peer_ranges,
        &peer_features,
    );
    let activation = SessionActivation::new(
        &fixture.root,
        initiator,
        responder,
        SessionAuthRole::Initiator,
        &fixture.responder_trust,
        [0x52; 32],
        [0x53; 32],
        &binding,
        TransportSecurityClass::InProcessTest,
        &initiator_proof,
        &responder_proof,
    );

    let mut session = LogicalSession::new();
    assert_eq!(session.state(), SessionState::Created);
    session.authenticate(activation).unwrap();

    let context = session.context().unwrap();
    assert_eq!(session.state(), SessionState::Active);
    assert_eq!(context.session_id(), expected_session_id);
    assert_eq!(context.owner_id(), fixture.owner_id);
    assert_eq!(
        context.local_device_id(),
        fixture.initiator_credential.device_id()
    );
    assert_eq!(
        context.peer_device_id(),
        fixture.responder_credential.device_id()
    );
    assert_eq!(context.peer_credential_epoch(), 5);
    assert_eq!(context.peer_trust_revision(), 0);
    assert_eq!(context.protocol_version(), ProtocolVersion::new(1, 2));
    assert_eq!(context.negotiated_features(), &[2, 3]);
    assert_eq!(
        context.transport_security_class(),
        TransportSecurityClass::InProcessTest
    );
    assert_eq!(context.next_send_sequence(), 0);
    assert_eq!(context.next_receive_sequence(), 0);
    assert!(context.negotiated_capabilities().is_empty());
}

#[test]
fn replayed_proof_on_fresh_nonce_fails_closed() {
    let fixture = Fixture::new();
    let local_ranges = Fixture::local_ranges();
    let peer_ranges = Fixture::peer_ranges();
    let local_features = Fixture::local_features();
    let peer_features = Fixture::peer_features();
    let binding = ChannelBinding::new("in-process-test", vec![0x61; 32]);
    let old_transcript = fixture.transcript(
        [0x62; 32],
        [0x63; 32],
        &binding,
        &local_ranges,
        &peer_ranges,
        &local_features,
        &peer_features,
    );
    let (initiator_proof, responder_proof) = fixture.proofs(&old_transcript);
    let activation = SessionActivation::new(
        &fixture.root,
        SessionHandshakeSide::new(
            &fixture.initiator_credential,
            &fixture.delegation,
            &local_ranges,
            &local_features,
        ),
        SessionHandshakeSide::new(
            &fixture.responder_credential,
            &fixture.delegation,
            &peer_ranges,
            &peer_features,
        ),
        SessionAuthRole::Initiator,
        &fixture.responder_trust,
        [0xee; 32],
        [0x63; 32],
        &binding,
        TransportSecurityClass::InProcessTest,
        &initiator_proof,
        &responder_proof,
    );

    let mut session = LogicalSession::new();
    assert_eq!(
        session.authenticate(activation),
        Err(SessionError::Auth(SessionAuthError::WrongProofTranscript))
    );
    assert_eq!(session.state(), SessionState::Closed);
    assert!(session.context().is_none());
}

#[test]
fn wrong_channel_binding_fails_closed() {
    let fixture = Fixture::new();
    let local_ranges = Fixture::local_ranges();
    let peer_ranges = Fixture::peer_ranges();
    let local_features = Fixture::local_features();
    let peer_features = Fixture::peer_features();
    let signed_binding = ChannelBinding::new("in-process-test", vec![0x71; 32]);
    let actual_binding = ChannelBinding::new("in-process-test", vec![0x72; 32]);
    let transcript = fixture.transcript(
        [0x73; 32],
        [0x74; 32],
        &signed_binding,
        &local_ranges,
        &peer_ranges,
        &local_features,
        &peer_features,
    );
    let (initiator_proof, responder_proof) = fixture.proofs(&transcript);
    let activation = SessionActivation::new(
        &fixture.root,
        SessionHandshakeSide::new(
            &fixture.initiator_credential,
            &fixture.delegation,
            &local_ranges,
            &local_features,
        ),
        SessionHandshakeSide::new(
            &fixture.responder_credential,
            &fixture.delegation,
            &peer_ranges,
            &peer_features,
        ),
        SessionAuthRole::Initiator,
        &fixture.responder_trust,
        [0x73; 32],
        [0x74; 32],
        &actual_binding,
        TransportSecurityClass::InProcessTest,
        &initiator_proof,
        &responder_proof,
    );

    let mut session = LogicalSession::new();
    assert_eq!(
        session.authenticate(activation),
        Err(SessionError::Auth(SessionAuthError::WrongProofTranscript))
    );
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn incompatible_protocol_or_required_feature_fails_before_activation() {
    let fixture = Fixture::new();
    let local_ranges = Fixture::local_ranges();
    let incompatible_ranges = [ProtocolRange::new(2, 0, 0).unwrap()];
    let local_features = Fixture::local_features();
    let peer_features = Fixture::peer_features();
    let binding = ChannelBinding::new("in-process-test", vec![0x81; 32]);
    let dummy_transcript = SessionAuthTranscriptV1::new(
        fixture.owner_id,
        &fixture.initiator_credential,
        [0x82; 32],
        &fixture.responder_credential,
        [0x83; 32],
        ProtocolVersion::new(1, 0),
        &[],
        binding.profile_id().as_bytes(),
        binding.bytes(),
    )
    .unwrap();
    let (dummy_initiator, dummy_responder) = fixture.proofs(&dummy_transcript);
    let mut session = LogicalSession::new();
    let activation = SessionActivation::new(
        &fixture.root,
        SessionHandshakeSide::new(
            &fixture.initiator_credential,
            &fixture.delegation,
            &local_ranges,
            &local_features,
        ),
        SessionHandshakeSide::new(
            &fixture.responder_credential,
            &fixture.delegation,
            &incompatible_ranges,
            &peer_features,
        ),
        SessionAuthRole::Initiator,
        &fixture.responder_trust,
        [0x82; 32],
        [0x83; 32],
        &binding,
        TransportSecurityClass::InProcessTest,
        &dummy_initiator,
        &dummy_responder,
    );
    assert_eq!(
        session.authenticate(activation),
        Err(SessionError::Protocol(
            VersionNegotiationError::IncompatibleProtocol
        ))
    );
    assert_eq!(session.state(), SessionState::Closed);

    let peer_ranges = Fixture::peer_ranges();
    let unsupported_required = FeatureSet::new(&[2, 3, 9], &[9]).unwrap();
    let mut session = LogicalSession::new();
    let activation = SessionActivation::new(
        &fixture.root,
        SessionHandshakeSide::new(
            &fixture.initiator_credential,
            &fixture.delegation,
            &local_ranges,
            &local_features,
        ),
        SessionHandshakeSide::new(
            &fixture.responder_credential,
            &fixture.delegation,
            &peer_ranges,
            &unsupported_required,
        ),
        SessionAuthRole::Initiator,
        &fixture.responder_trust,
        [0x82; 32],
        [0x83; 32],
        &binding,
        TransportSecurityClass::InProcessTest,
        &dummy_initiator,
        &dummy_responder,
    );
    assert_eq!(
        session.authenticate(activation),
        Err(SessionError::Feature(
            FeatureNegotiationError::UnsupportedRequiredFeature(9)
        ))
    );
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn peer_trust_identity_and_accepted_credential_epoch_are_activation_gates() {
    let fixture = Fixture::new();
    let local_ranges = Fixture::local_ranges();
    let peer_ranges = Fixture::peer_ranges();
    let local_features = Fixture::local_features();
    let peer_features = Fixture::peer_features();
    let binding = ChannelBinding::new("in-process-test", vec![0x91; 32]);
    let transcript = fixture.transcript(
        [0x92; 32],
        [0x93; 32],
        &binding,
        &local_ranges,
        &peer_ranges,
        &local_features,
        &peer_features,
    );
    let (initiator_proof, responder_proof) = fixture.proofs(&transcript);

    let wrong_device_trust = TrustRecord::trusted(
        fixture.owner_id,
        DeviceId::from_bytes([0xaa; 32]),
        fixture.responder_credential.credential_epoch(),
        TransitionId::from_bytes([0xab; 32]),
    );
    let mut session = LogicalSession::new();
    let activation = SessionActivation::new(
        &fixture.root,
        SessionHandshakeSide::new(
            &fixture.initiator_credential,
            &fixture.delegation,
            &local_ranges,
            &local_features,
        ),
        SessionHandshakeSide::new(
            &fixture.responder_credential,
            &fixture.delegation,
            &peer_ranges,
            &peer_features,
        ),
        SessionAuthRole::Initiator,
        &wrong_device_trust,
        [0x92; 32],
        [0x93; 32],
        &binding,
        TransportSecurityClass::InProcessTest,
        &initiator_proof,
        &responder_proof,
    );
    assert_eq!(
        session.authenticate(activation),
        Err(SessionError::PeerTrustMismatch)
    );
    assert_eq!(session.state(), SessionState::Closed);

    let stale_trust = TrustRecord::trusted(
        fixture.owner_id,
        fixture.responder_credential.device_id(),
        fixture.responder_credential.credential_epoch() - 1,
        TransitionId::from_bytes([0xac; 32]),
    );
    let mut session = LogicalSession::new();
    let activation = SessionActivation::new(
        &fixture.root,
        SessionHandshakeSide::new(
            &fixture.initiator_credential,
            &fixture.delegation,
            &local_ranges,
            &local_features,
        ),
        SessionHandshakeSide::new(
            &fixture.responder_credential,
            &fixture.delegation,
            &peer_ranges,
            &peer_features,
        ),
        SessionAuthRole::Initiator,
        &stale_trust,
        [0x92; 32],
        [0x93; 32],
        &binding,
        TransportSecurityClass::InProcessTest,
        &initiator_proof,
        &responder_proof,
    );
    assert_eq!(
        session.authenticate(activation),
        Err(SessionError::PeerCredentialEpochMismatch)
    );
    assert_eq!(session.state(), SessionState::Closed);
}

fn active_session(fixture: &Fixture) -> LogicalSession {
    let local_ranges = Fixture::local_ranges();
    let peer_ranges = Fixture::peer_ranges();
    let local_features = Fixture::local_features();
    let peer_features = Fixture::peer_features();
    let binding = ChannelBinding::new("in-process-test", vec![0xa1; 32]);
    let transcript = fixture.transcript(
        [0xa2; 32],
        [0xa3; 32],
        &binding,
        &local_ranges,
        &peer_ranges,
        &local_features,
        &peer_features,
    );
    let (initiator_proof, responder_proof) = fixture.proofs(&transcript);
    let activation = SessionActivation::new(
        &fixture.root,
        SessionHandshakeSide::new(
            &fixture.initiator_credential,
            &fixture.delegation,
            &local_ranges,
            &local_features,
        ),
        SessionHandshakeSide::new(
            &fixture.responder_credential,
            &fixture.delegation,
            &peer_ranges,
            &peer_features,
        ),
        SessionAuthRole::Initiator,
        &fixture.responder_trust,
        [0xa2; 32],
        [0xa3; 32],
        &binding,
        TransportSecurityClass::InProcessTest,
        &initiator_proof,
        &responder_proof,
    );
    let mut session = LogicalSession::new();
    session.authenticate(activation).unwrap();
    session
}

#[test]
fn capability_exchange_is_post_auth_only_and_is_not_policy_authority() {
    let fixture = Fixture::new();
    let local = vec![
        LocalCapability::new(
            CapabilityId::parse("clipboard.read").unwrap(),
            CapabilityVersionRange::new(1, 0, 3).unwrap(),
            true,
        ),
        LocalCapability::new(
            CapabilityId::parse("files.transfer").unwrap(),
            CapabilityVersionRange::new(1, 0, 0).unwrap(),
            false,
        ),
        LocalCapability::new(
            CapabilityId::parse("clipboard.write").unwrap(),
            CapabilityVersionRange::new(2, 0, 1).unwrap(),
            true,
        ),
    ];
    let peer = CapabilityAdvertisement::new(vec![
        CapabilityAdvertisementEntry::new(
            CapabilityId::parse("clipboard.read").unwrap(),
            CapabilityVersion::new(1, 1),
            CapabilityVersion::new(1, 2),
            true,
        )
        .unwrap(),
        CapabilityAdvertisementEntry::new(
            CapabilityId::parse("files.transfer").unwrap(),
            CapabilityVersion::new(1, 0),
            CapabilityVersion::new(1, 0),
            true,
        )
        .unwrap(),
        CapabilityAdvertisementEntry::new(
            CapabilityId::parse("clipboard.write").unwrap(),
            CapabilityVersion::new(1, 0),
            CapabilityVersion::new(1, 0),
            true,
        )
        .unwrap(),
    ])
    .unwrap();

    let mut unauthenticated = LogicalSession::new();
    assert_eq!(
        unauthenticated.negotiate_capabilities(&local, &peer),
        Err(SessionError::InvalidState)
    );

    let mut session = active_session(&fixture);
    session.negotiate_capabilities(&local, &peer).unwrap();
    let context = session.context().unwrap();
    let negotiated = context.negotiated_capabilities();
    assert_eq!(negotiated.len(), 1);
    assert_eq!(negotiated[0].capability_id().as_str(), "clipboard.read");
    assert_eq!(negotiated[0].version(), CapabilityVersion::new(1, 2));

    let policy = PolicyState::new();
    let auth_context = AuthorizationContext::new(
        context.peer_device_id(),
        context.local_device_id(),
        context.session_id(),
        negotiated[0].capability_id().clone(),
        negotiated[0].version(),
        OperationName::parse("get").unwrap(),
        TrustState::Trusted,
        context.peer_trust_revision(),
        local[0].clone(),
        NetworkClass::Local,
    );
    let decision = policy.evaluate(&auth_context);
    assert_eq!(decision.effect(), DecisionEffect::Deny);
    assert_eq!(decision.reason(), DecisionReason::NoMatchingRule);
    assert!(decision.into_grant().is_none());
}

#[test]
fn close_and_revocation_transitions_are_explicit_and_fail_invalid_transitions() {
    let fixture = Fixture::new();
    let mut session = active_session(&fixture);
    session.begin_close().unwrap();
    assert_eq!(session.state(), SessionState::Closing);
    session.finish_close().unwrap();
    assert_eq!(session.state(), SessionState::Closed);
    assert_eq!(session.begin_close(), Err(SessionError::InvalidState));

    let mut revoked = active_session(&fixture);
    revoked.revoke().unwrap();
    assert_eq!(revoked.state(), SessionState::Revoked);
    assert_eq!(revoked.begin_close(), Err(SessionError::InvalidState));
    revoked.finish_close().unwrap();
    assert_eq!(revoked.state(), SessionState::Closed);
}