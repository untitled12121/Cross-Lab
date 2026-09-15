use std::num::NonZeroUsize;

use crosslab_core::{
    ChannelBinding, LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionHandshakeSide, StreamAdmission, StreamAdmissionError, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    AuthorizationContext, AuthorizedOperation, CapabilityId, CapabilityVersion,
    CapabilityVersionRange, LocalCapability, NetworkClass, OperationError, OperationName,
    PairingTrustTransition, PolicyRule, PolicyState, RuleEffect, RuleId, TransitionId, TrustRecord,
    TrustState, TrustTransition, UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, DataStreamOpen, FeatureSet,
    ProtocolRange, ProtocolVersion, StreamDirection, StreamId,
};

struct Fixture {
    authority: OwnerAuthorityState,
    root_key: SigningKey,
    issuer_key: SigningKey,
    session: LogicalSession,
    peer_trust: TrustRecord,
    policy: PolicyState,
    grant: crosslab_policy::AuthorizationGrant,
    capability: CapabilityId,
    version: CapabilityVersion,
    operation: OperationName,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x70; 32]);
        let root_key = SigningKey::from_secret_bytes([0x71; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x72; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let local_key = SigningKey::from_secret_bytes([0x73; 32]);
        let peer_key = SigningKey::from_secret_bytes([0x74; 32]);
        let local_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x75; 32]),
            &local_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let peer_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x76; 32]),
            &peer_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let pairing = PairingTrustTransition::issue(
            &peer_credential,
            TransitionId::from_bytes([0x77; 32]),
            [0x78; 32],
            &authority,
            &issuer_key,
        )
        .unwrap();
        let peer_trust = pairing.establish(&peer_credential, &authority).unwrap();

        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = ChannelBinding::new("in-process-test", vec![0x79; 32]);
        let transcript = SessionAuthTranscriptV1::new(
            owner_id,
            &local_credential,
            [0x7a; 32],
            &peer_credential,
            [0x7b; 32],
            ProtocolVersion::new(1, 0),
            &[],
            binding.profile_id().as_bytes(),
            binding.bytes(),
        )
        .unwrap();
        let local_proof = transcript
            .create_proof(SessionAuthRole::Initiator, &local_key)
            .unwrap();
        let peer_proof = transcript
            .create_proof(SessionAuthRole::Responder, &peer_key)
            .unwrap();
        let mut session = LogicalSession::new();
        session
            .authenticate(SessionActivation::new(
                &authority,
                SessionHandshakeSide::new(&local_credential, &ranges, &features),
                SessionHandshakeSide::new(&peer_credential, &ranges, &features),
                SessionAuthRole::Initiator,
                &peer_trust,
                [0x7a; 32],
                [0x7b; 32],
                &binding,
                TransportSecurityClass::InProcessTest,
                &local_proof,
                &peer_proof,
            ))
            .unwrap();

        let capability = CapabilityId::parse("files.transfer").unwrap();
        let version = CapabilityVersion::new(1, 0);
        let operation = OperationName::parse("send").unwrap();
        let local_capability = LocalCapability::new(
            capability.clone(),
            CapabilityVersionRange::new(1, 0, 0).unwrap(),
            true,
        );
        let advertisement = CapabilityAdvertisement::new(vec![
            CapabilityAdvertisementEntry::new(capability.clone(), version, version, true).unwrap(),
        ])
        .unwrap();
        session
            .negotiate_capabilities(std::slice::from_ref(&local_capability), &advertisement)
            .unwrap();

        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([0x7c; 32]),
                peer_credential.device_id(),
                capability.clone(),
                operation.clone(),
                RuleEffect::Allow,
            ))
            .unwrap();
        let context = AuthorizationContext::new(
            peer_credential.device_id(),
            local_credential.device_id(),
            session.context().unwrap().session_id(),
            capability.clone(),
            version,
            operation.clone(),
            TrustState::Trusted,
            peer_trust.trust_revision(),
            local_capability,
            NetworkClass::Local,
        );
        let grant = policy.evaluate(&context).into_grant().unwrap();

        Self {
            authority,
            root_key,
            issuer_key,
            session,
            peer_trust,
            policy,
            grant,
            capability,
            version,
            operation,
        }
    }

    fn operation(&self) -> AuthorizedOperation {
        AuthorizedOperation::issue(self.grant.clone(), 10, 20, UsePolicy::SingleStream).unwrap()
    }

    fn open(&self, operation: &AuthorizedOperation, stream_byte: u8) -> DataStreamOpen {
        DataStreamOpen::new(
            self.session.context().unwrap().session_id(),
            StreamId::from_bytes([stream_byte; 16]),
            operation.id(),
            self.capability.clone(),
            self.version,
            self.operation.clone(),
            StreamDirection::SourceToDestination,
            0,
        )
    }
}

#[test]
fn admission_derives_current_policy_revision_from_local_state() {
    let mut fixture = Fixture::new();
    let operation = fixture.operation();
    let open = fixture.open(&operation, 0x80);
    let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());
    admission.register_operation(operation).unwrap();

    fixture
        .policy
        .insert(PolicyRule::new(
            RuleId::from_bytes([0x81; 32]),
            fixture.peer_trust.device_id(),
            CapabilityId::parse("clipboard.read").unwrap(),
            OperationName::parse("get").unwrap(),
            RuleEffect::Deny,
        ))
        .unwrap();

    assert_eq!(
        admission.admit_inbound(
            &fixture.session,
            &open,
            15,
            &fixture.peer_trust,
            &fixture.policy,
        ),
        Err(StreamAdmissionError::Operation(
            OperationError::PolicyRevisionChanged
        ))
    );
}

#[test]
fn admission_rejects_locally_revoked_peer_trust() {
    let fixture = Fixture::new();
    let operation = fixture.operation();
    let open = fixture.open(&operation, 0x82);
    let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());
    admission.register_operation(operation).unwrap();

    let transition = TrustTransition::issue_root_revocation(
        &fixture.peer_trust,
        TransitionId::from_bytes([0x83; 32]),
        &fixture.authority,
        &fixture.root_key,
    )
    .unwrap();
    let mut revoked = fixture.peer_trust;
    transition
        .apply_root(&mut revoked, &fixture.authority)
        .unwrap();

    assert_eq!(
        admission.admit_inbound(&fixture.session, &open, 15, &revoked, &fixture.policy),
        Err(StreamAdmissionError::PeerNotTrusted)
    );
}

#[test]
fn admission_rejects_trust_for_a_different_peer_device() {
    let fixture = Fixture::new();
    let operation = fixture.operation();
    let open = fixture.open(&operation, 0x84);
    let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());
    admission.register_operation(operation).unwrap();

    let other_key = SigningKey::from_secret_bytes([0x85; 32]);
    let other_credential = DeviceCredential::issue(
        fixture.authority.root().owner_id(),
        DeviceId::from_bytes([0x86; 32]),
        &other_key,
        0,
        &fixture.authority,
        &fixture.issuer_key,
    )
    .unwrap();
    let pairing = PairingTrustTransition::issue(
        &other_credential,
        TransitionId::from_bytes([0x87; 32]),
        [0x88; 32],
        &fixture.authority,
        &fixture.issuer_key,
    )
    .unwrap();
    let other_trust = pairing
        .establish(&other_credential, &fixture.authority)
        .unwrap();

    assert_eq!(
        admission.admit_inbound(&fixture.session, &open, 15, &other_trust, &fixture.policy,),
        Err(StreamAdmissionError::PeerTrustMismatch)
    );
}
