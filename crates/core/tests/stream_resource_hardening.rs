use std::num::NonZeroUsize;

use crosslab_core::{
    ChannelBinding, LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionHandshakeSide, StreamAdmission, StreamAdmissionError, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    AuthorizationContext, AuthorizedOperation, CapabilityId, CapabilityVersion,
    CapabilityVersionRange, LocalCapability, NetworkClass, OperationError, OperationName,
    OperationState, PolicyRule, PolicyState, RuleEffect, RuleId, TransitionId, TrustRecord,
    TrustState, UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, DataStreamOpen, FeatureSet,
    ProtocolRange, ProtocolVersion, StreamDirection, StreamId,
};

struct Fixture {
    session: LogicalSession,
    capability: CapabilityId,
    version: CapabilityVersion,
    operation: OperationName,
    trust_revision: u64,
    policy_revision: u64,
    grant: crosslab_policy::AuthorizationGrant,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0xe3; 32]);
        let root_key = SigningKey::from_secret_bytes([0xe4; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0xe5; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let initiator_key = SigningKey::from_secret_bytes([0xe6; 32]);
        let responder_key = SigningKey::from_secret_bytes([0xe7; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0xe8; 32]),
            &initiator_key,
            1,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0xe9; 32]),
            &responder_key,
            1,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let peer_trust = TrustRecord::trusted(
            owner_id,
            responder_credential.device_id(),
            responder_credential.credential_epoch(),
            TransitionId::from_bytes([0xea; 32]),
        );
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = ChannelBinding::new("in-process-test", vec![0xeb; 32]);
        let transcript = SessionAuthTranscriptV1::new(
            owner_id,
            &initiator_credential,
            [0xec; 32],
            &responder_credential,
            [0xed; 32],
            ProtocolVersion::new(1, 0),
            &[],
            binding.profile_id().as_bytes(),
            binding.bytes(),
        )
        .unwrap();
        let initiator_proof = transcript
            .create_proof(SessionAuthRole::Initiator, &initiator_key)
            .unwrap();
        let responder_proof = transcript
            .create_proof(SessionAuthRole::Responder, &responder_key)
            .unwrap();
        let mut session = LogicalSession::new();
        session
            .authenticate(SessionActivation::new(
                &root,
                SessionHandshakeSide::new(&initiator_credential, &delegation, &ranges, &features),
                SessionHandshakeSide::new(&responder_credential, &delegation, &ranges, &features),
                SessionAuthRole::Initiator,
                &peer_trust,
                [0xec; 32],
                [0xed; 32],
                &binding,
                TransportSecurityClass::InProcessTest,
                &initiator_proof,
                &responder_proof,
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

        let source = responder_credential.device_id();
        let destination = initiator_credential.device_id();
        let trust_revision = peer_trust.trust_revision();
        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([0xee; 32]),
                source,
                capability.clone(),
                operation.clone(),
                RuleEffect::Allow,
            ))
            .unwrap();
        let context = AuthorizationContext::new(
            source,
            destination,
            session.context().unwrap().session_id(),
            capability.clone(),
            version,
            operation.clone(),
            TrustState::Trusted,
            trust_revision,
            local_capability,
            NetworkClass::Local,
        );
        let grant = policy.evaluate(&context).into_grant().unwrap();

        Self {
            session,
            capability,
            version,
            operation,
            trust_revision,
            policy_revision: policy.revision(),
            grant,
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

    fn admit(
        &self,
        admission: &mut StreamAdmission,
        open: &DataStreamOpen,
        now: u64,
    ) -> Result<crosslab_core::AdmittedStream, StreamAdmissionError> {
        admission.admit_inbound(
            &self.session,
            open,
            now,
            self.trust_revision,
            self.policy_revision,
        )
    }
}

#[test]
fn completed_operation_releases_registration_capacity() {
    let fixture = Fixture::new();
    let mut admission = StreamAdmission::new(NonZeroUsize::new(1).unwrap());
    let first = fixture.operation();
    let first_open = fixture.open(&first, 0xef);
    admission.register_operation(first).unwrap();
    fixture.admit(&mut admission, &first_open, 15).unwrap();
    admission.finish_stream(first_open.stream_id()).unwrap();

    assert!(admission.register_operation(fixture.operation()).is_ok());
}

#[test]
fn operation_that_expires_during_admission_releases_capacity() {
    let fixture = Fixture::new();
    let mut admission = StreamAdmission::new(NonZeroUsize::new(1).unwrap());
    let expired = fixture.operation();
    let expired_open = fixture.open(&expired, 0xf0);
    admission.register_operation(expired).unwrap();

    assert_eq!(
        fixture.admit(&mut admission, &expired_open, 20),
        Err(StreamAdmissionError::Operation(OperationError::Inactive(
            OperationState::Expired
        )))
    );
    assert!(admission.register_operation(fixture.operation()).is_ok());
}

#[test]
fn cancel_all_releases_registered_operation_state() {
    let fixture = Fixture::new();
    let mut admission = StreamAdmission::new(NonZeroUsize::new(1).unwrap());
    admission.register_operation(fixture.operation()).unwrap();

    admission.cancel_all();

    assert!(admission.register_operation(fixture.operation()).is_ok());
}
