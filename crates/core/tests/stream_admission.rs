use std::num::{NonZeroU32, NonZeroUsize};

use crosslab_core::{
    AdmittedStream, ChannelBinding, LogicalSession, SessionActivation, SessionAuthRole,
    SessionAuthTranscriptV1, SessionHandshakeSide, StreamAdmission, StreamAdmissionError,
    TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    AuthorizationContext, AuthorizedOperation, CapabilityId, CapabilityVersion,
    CapabilityVersionRange, LocalCapability, NetworkClass, OperationError, OperationName,
    OperationState, PolicyRule, PolicyState, RuleEffect, RuleId, SessionId, TransitionId,
    TrustRecord, TrustState, UsePolicy,
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
        let initiator_key = SigningKey::from_secret_bytes([0x13; 32]);
        let responder_key = SigningKey::from_secret_bytes([0x14; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x15; 32]),
            &initiator_key,
            1,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x16; 32]),
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
            TransitionId::from_bytes([0x17; 32]),
        );
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = ChannelBinding::new("in-process-test", vec![0x18; 32]);
        let transcript = SessionAuthTranscriptV1::new(
            owner_id,
            &initiator_credential,
            [0x19; 32],
            &responder_credential,
            [0x1a; 32],
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
        let activation = SessionActivation::new(
            &root,
            SessionHandshakeSide::new(&initiator_credential, &delegation, &ranges, &features),
            SessionHandshakeSide::new(&responder_credential, &delegation, &ranges, &features),
            SessionAuthRole::Initiator,
            &peer_trust,
            [0x19; 32],
            [0x1a; 32],
            &binding,
            TransportSecurityClass::InProcessTest,
            &initiator_proof,
            &responder_proof,
        );
        let mut session = LogicalSession::new();
        session.authenticate(activation).unwrap();

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
                RuleId::from_bytes([0x1b; 32]),
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

    fn operation(&self, use_policy: UsePolicy) -> AuthorizedOperation {
        AuthorizedOperation::issue(self.grant.clone(), 10, 20, use_policy).unwrap()
    }

    fn open(
        &self,
        operation_id: crosslab_policy::OperationId,
        direction: StreamDirection,
        stream_index: u32,
        stream_byte: u8,
    ) -> DataStreamOpen {
        DataStreamOpen::new(
            self.session.context().unwrap().session_id(),
            StreamId::from_bytes([stream_byte; 16]),
            operation_id,
            self.capability.clone(),
            self.version,
            self.operation.clone(),
            direction,
            stream_index,
        )
    }

    fn admit(
        &self,
        admission: &mut StreamAdmission,
        open: &DataStreamOpen,
    ) -> Result<AdmittedStream, StreamAdmissionError> {
        self.admit_with_revisions(admission, open, self.trust_revision, self.policy_revision)
    }

    fn admit_with_revisions(
        &self,
        admission: &mut StreamAdmission,
        open: &DataStreamOpen,
        trust_revision: u64,
        policy_revision: u64,
    ) -> Result<AdmittedStream, StreamAdmissionError> {
        admission.admit_inbound(&self.session, open, 15, trust_revision, policy_revision)
    }
}

#[test]
fn valid_single_stream_is_admitted_once_and_retired_after_finish() {
    let fixture = Fixture::new();
    let operation = fixture.operation(UsePolicy::SingleStream);
    let operation_id = operation.id();
    let open = fixture.open(operation_id, StreamDirection::SourceToDestination, 0, 0x21);
    let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());
    admission.register_operation(operation).unwrap();

    let admitted = fixture.admit(&mut admission, &open).unwrap();
    assert_eq!(admitted.stream_id(), open.stream_id());
    assert_eq!(admitted.operation_id(), operation_id);
    assert_eq!(admitted.stream_index(), 0);
    assert_eq!(admission.active_stream_count(), 1);

    admission.finish_stream(open.stream_id()).unwrap();
    assert_eq!(admission.active_stream_count(), 0);
    assert_eq!(
        fixture.admit(&mut admission, &open),
        Err(StreamAdmissionError::OperationNotFound)
    );
}

#[test]
fn missing_operation_wrong_session_and_unnegotiated_capability_fail() {
    let fixture = Fixture::new();
    let operation = fixture.operation(UsePolicy::SingleStream);
    let operation_id = operation.id();
    let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());

    let missing = fixture.open(operation_id, StreamDirection::SourceToDestination, 0, 0x22);
    assert_eq!(
        fixture.admit(&mut admission, &missing),
        Err(StreamAdmissionError::OperationNotFound)
    );

    admission.register_operation(operation).unwrap();
    let wrong_session = DataStreamOpen::new(
        SessionId::from_bytes([0x23; 32]),
        StreamId::from_bytes([0x24; 16]),
        operation_id,
        fixture.capability.clone(),
        fixture.version,
        fixture.operation.clone(),
        StreamDirection::SourceToDestination,
        0,
    );
    assert_eq!(
        fixture.admit(&mut admission, &wrong_session),
        Err(StreamAdmissionError::InvalidSession)
    );

    let unsupported_capability = DataStreamOpen::new(
        fixture.session.context().unwrap().session_id(),
        StreamId::from_bytes([0x25; 16]),
        operation_id,
        CapabilityId::parse("clipboard.read").unwrap(),
        fixture.version,
        fixture.operation.clone(),
        StreamDirection::SourceToDestination,
        0,
    );
    assert_eq!(
        fixture.admit(&mut admission, &unsupported_capability),
        Err(StreamAdmissionError::CapabilityNotNegotiated)
    );

    let unsupported_version = DataStreamOpen::new(
        fixture.session.context().unwrap().session_id(),
        StreamId::from_bytes([0x26; 16]),
        operation_id,
        fixture.capability.clone(),
        CapabilityVersion::new(1, 1),
        fixture.operation.clone(),
        StreamDirection::SourceToDestination,
        0,
    );
    assert_eq!(
        fixture.admit(&mut admission, &unsupported_version),
        Err(StreamAdmissionError::CapabilityNotNegotiated)
    );
}

#[test]
fn operation_direction_and_stream_index_are_bound_before_reservation() {
    let fixture = Fixture::new();
    let operation = fixture.operation(UsePolicy::SingleStream);
    let operation_id = operation.id();
    let mut admission = StreamAdmission::new(NonZeroUsize::new(8).unwrap());
    admission.register_operation(operation).unwrap();

    let wrong_operation = DataStreamOpen::new(
        fixture.session.context().unwrap().session_id(),
        StreamId::from_bytes([0x27; 16]),
        operation_id,
        fixture.capability.clone(),
        fixture.version,
        OperationName::parse("receive").unwrap(),
        StreamDirection::SourceToDestination,
        0,
    );
    assert_eq!(
        fixture.admit(&mut admission, &wrong_operation),
        Err(StreamAdmissionError::Operation(
            OperationError::BindingMismatch
        ))
    );

    let wrong_direction = fixture.open(operation_id, StreamDirection::DestinationToSource, 0, 0x28);
    assert_eq!(
        fixture.admit(&mut admission, &wrong_direction),
        Err(StreamAdmissionError::Operation(
            OperationError::BindingMismatch
        ))
    );

    let invalid_index = fixture.open(operation_id, StreamDirection::SourceToDestination, 1, 0x29);
    assert_eq!(
        fixture.admit(&mut admission, &invalid_index),
        Err(StreamAdmissionError::InvalidStreamIndex)
    );
}

#[test]
fn terminal_expired_and_revision_invalid_operations_fail() {
    let fixture = Fixture::new();

    for (state, stream_byte) in [
        (OperationState::Cancelled, 0x30),
        (OperationState::Revoked, 0x31),
        (OperationState::Consumed, 0x32),
    ] {
        let mut operation = fixture.operation(UsePolicy::SingleStream);
        match state {
            OperationState::Cancelled => operation.cancel(),
            OperationState::Revoked => operation.revoke(),
            OperationState::Consumed => operation.consume(),
            _ => unreachable!(),
        }
        let open = fixture.open(
            operation.id(),
            StreamDirection::SourceToDestination,
            0,
            stream_byte,
        );
        let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());
        admission.register_operation(operation).unwrap();
        assert_eq!(
            fixture.admit(&mut admission, &open),
            Err(StreamAdmissionError::Operation(OperationError::Inactive(
                state
            )))
        );
    }

    let expired = fixture.operation(UsePolicy::SingleStream);
    let expired_open = fixture.open(expired.id(), StreamDirection::SourceToDestination, 0, 0x33);
    let mut expired_admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());
    expired_admission.register_operation(expired).unwrap();
    assert_eq!(
        expired_admission.admit_inbound(
            &fixture.session,
            &expired_open,
            20,
            fixture.trust_revision,
            fixture.policy_revision,
        ),
        Err(StreamAdmissionError::Operation(OperationError::Inactive(
            OperationState::Expired
        )))
    );

    for (trust_revision, policy_revision, expected, stream_byte) in [
        (
            fixture.trust_revision + 1,
            fixture.policy_revision,
            OperationError::TrustRevisionChanged,
            0x34,
        ),
        (
            fixture.trust_revision,
            fixture.policy_revision + 1,
            OperationError::PolicyRevisionChanged,
            0x35,
        ),
    ] {
        let operation = fixture.operation(UsePolicy::SingleStream);
        let open = fixture.open(
            operation.id(),
            StreamDirection::SourceToDestination,
            0,
            stream_byte,
        );
        let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());
        admission.register_operation(operation).unwrap();
        assert_eq!(
            fixture.admit_with_revisions(&mut admission, &open, trust_revision, policy_revision),
            Err(StreamAdmissionError::Operation(expected))
        );
    }
}

#[test]
fn multi_stream_indices_are_unique_bounded_and_need_not_arrive_in_order() {
    let fixture = Fixture::new();
    let operation = fixture.operation(UsePolicy::MultiStream {
        max_streams: NonZeroU32::new(2).unwrap(),
    });
    let operation_id = operation.id();
    let mut admission = StreamAdmission::new(NonZeroUsize::new(2).unwrap());
    admission.register_operation(operation).unwrap();

    let index_one = fixture.open(operation_id, StreamDirection::SourceToDestination, 1, 0x40);
    fixture.admit(&mut admission, &index_one).unwrap();

    let duplicate_id = fixture.open(operation_id, StreamDirection::SourceToDestination, 0, 0x40);
    assert_eq!(
        fixture.admit(&mut admission, &duplicate_id),
        Err(StreamAdmissionError::DuplicateStreamId)
    );

    let duplicate_index = fixture.open(operation_id, StreamDirection::SourceToDestination, 1, 0x41);
    assert_eq!(
        fixture.admit(&mut admission, &duplicate_index),
        Err(StreamAdmissionError::DuplicateStreamIndex)
    );

    let index_zero = fixture.open(operation_id, StreamDirection::SourceToDestination, 0, 0x42);
    fixture.admit(&mut admission, &index_zero).unwrap();

    let out_of_range = fixture.open(operation_id, StreamDirection::SourceToDestination, 2, 0x43);
    assert_eq!(
        fixture.admit(&mut admission, &out_of_range),
        Err(StreamAdmissionError::InvalidStreamIndex)
    );

    let extra_operation = fixture.operation(UsePolicy::SingleStream);
    assert_eq!(
        admission.register_operation(extra_operation),
        Err(StreamAdmissionError::ResourceLimit)
    );
}
