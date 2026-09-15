use std::num::{NonZeroU32, NonZeroUsize};

use crosslab_core::{
    AdmittedStream, ChannelBinding, LogicalSession, SessionActivation, SessionAuthRole,
    SessionAuthTranscriptV1, SessionHandshakeSide, StreamAdmission, StreamAdmissionError,
    TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    AuthorizationContext, AuthorizedOperation, CapabilityId, CapabilityVersion,
    CapabilityVersionRange, LocalCapability, NetworkClass, OperationError, OperationName,
    OperationState, PairingTrustTransition, PolicyRule, PolicyState, RuleEffect, RuleId, SessionId,
    TransitionId, TrustRecord, TrustState, UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, DataStreamOpen, FeatureSet,
    ProtocolRange, ProtocolVersion, StreamDirection, StreamId,
};

struct Fixture {
    session: LogicalSession,
    peer_trust: TrustRecord,
    policy: PolicyState,
    capability: CapabilityId,
    version: CapabilityVersion,
    operation: OperationName,
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
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let initiator_key = SigningKey::from_secret_bytes([0x13; 32]);
        let responder_key = SigningKey::from_secret_bytes([0x14; 32]);
        let initiator_credential = DeviceCredential::issue_current(
            owner_id,
            DeviceId::from_bytes([0x15; 32]),
            &initiator_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue_current(
            owner_id,
            DeviceId::from_bytes([0x16; 32]),
            &responder_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let pairing = PairingTrustTransition::issue_current(
            &responder_credential,
            TransitionId::from_bytes([0x17; 32]),
            [0x18; 32],
            &authority,
            &issuer_key,
        )
        .unwrap();
        let peer_trust = pairing
            .establish_current(&responder_credential, &authority)
            .unwrap();

        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = ChannelBinding::new("in-process-test", vec![0x19; 32]);
        let transcript = SessionAuthTranscriptV1::new(
            owner_id,
            &initiator_credential,
            [0x1a; 32],
            &responder_credential,
            [0x1b; 32],
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
                &authority,
                SessionHandshakeSide::new(&initiator_credential, &ranges, &features),
                SessionHandshakeSide::new(&responder_credential, &ranges, &features),
                SessionAuthRole::Initiator,
                &peer_trust,
                [0x1a; 32],
                [0x1b; 32],
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

        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([0x1c; 32]),
                responder_credential.device_id(),
                capability.clone(),
                operation.clone(),
                RuleEffect::Allow,
            ))
            .unwrap();
        let context = AuthorizationContext::new(
            responder_credential.device_id(),
            initiator_credential.device_id(),
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
            session,
            peer_trust,
            policy,
            capability,
            version,
            operation,
            grant,
        }
    }

    fn authorized_operation(&self, use_policy: UsePolicy) -> AuthorizedOperation {
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
        now: u64,
    ) -> Result<AdmittedStream, StreamAdmissionError> {
        admission.admit_inbound(&self.session, open, now, &self.peer_trust, &self.policy)
    }
}

#[test]
fn valid_single_stream_is_admitted_once_and_retired_after_finish() {
    let fixture = Fixture::new();
    let operation = fixture.authorized_operation(UsePolicy::SingleStream);
    let operation_id = operation.id();
    let open = fixture.open(operation_id, StreamDirection::SourceToDestination, 0, 0x21);
    let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());
    admission.register_operation(operation).unwrap();

    let admitted = fixture.admit(&mut admission, &open, 15).unwrap();
    assert_eq!(admitted.stream_id(), open.stream_id());
    assert_eq!(admitted.operation_id(), operation_id);
    assert_eq!(admission.active_stream_count(), 1);

    admission.finish_stream(open.stream_id()).unwrap();
    assert_eq!(admission.active_stream_count(), 0);
    assert_eq!(
        fixture.admit(&mut admission, &open, 15),
        Err(StreamAdmissionError::OperationNotFound)
    );
}

#[test]
fn missing_operation_wrong_session_and_unnegotiated_capability_fail() {
    let fixture = Fixture::new();
    let operation = fixture.authorized_operation(UsePolicy::SingleStream);
    let operation_id = operation.id();
    let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());

    let missing = fixture.open(operation_id, StreamDirection::SourceToDestination, 0, 0x22);
    assert_eq!(
        fixture.admit(&mut admission, &missing, 15),
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
        fixture.admit(&mut admission, &wrong_session, 15),
        Err(StreamAdmissionError::InvalidSession)
    );

    let unsupported = DataStreamOpen::new(
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
        fixture.admit(&mut admission, &unsupported, 15),
        Err(StreamAdmissionError::CapabilityNotNegotiated)
    );
}

#[test]
fn operation_direction_and_stream_index_are_bound_before_reservation() {
    let fixture = Fixture::new();
    let operation = fixture.authorized_operation(UsePolicy::SingleStream);
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
        fixture.admit(&mut admission, &wrong_operation, 15),
        Err(StreamAdmissionError::Operation(
            OperationError::BindingMismatch
        ))
    );

    let wrong_direction = fixture.open(operation_id, StreamDirection::DestinationToSource, 0, 0x28);
    assert_eq!(
        fixture.admit(&mut admission, &wrong_direction, 15),
        Err(StreamAdmissionError::Operation(
            OperationError::BindingMismatch
        ))
    );

    let invalid_index = fixture.open(operation_id, StreamDirection::SourceToDestination, 1, 0x29);
    assert_eq!(
        fixture.admit(&mut admission, &invalid_index, 15),
        Err(StreamAdmissionError::InvalidStreamIndex)
    );
}

#[test]
fn terminal_and_expired_operations_fail_closed() {
    let fixture = Fixture::new();

    for (state, stream_byte) in [
        (OperationState::Cancelled, 0x30),
        (OperationState::Revoked, 0x31),
        (OperationState::Consumed, 0x32),
    ] {
        let mut operation = fixture.authorized_operation(UsePolicy::SingleStream);
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
            fixture.admit(&mut admission, &open, 15),
            Err(StreamAdmissionError::Operation(OperationError::Inactive(
                state
            )))
        );
    }

    let expired = fixture.authorized_operation(UsePolicy::SingleStream);
    let expired_open = fixture.open(expired.id(), StreamDirection::SourceToDestination, 0, 0x33);
    let mut admission = StreamAdmission::new(NonZeroUsize::new(4).unwrap());
    admission.register_operation(expired).unwrap();
    assert_eq!(
        fixture.admit(&mut admission, &expired_open, 20),
        Err(StreamAdmissionError::Operation(OperationError::Inactive(
            OperationState::Expired
        )))
    );
}

#[test]
fn multi_stream_indices_are_unique_and_bounded() {
    let fixture = Fixture::new();
    let operation = fixture.authorized_operation(UsePolicy::MultiStream {
        max_streams: NonZeroU32::new(2).unwrap(),
    });
    let operation_id = operation.id();
    let mut admission = StreamAdmission::new(NonZeroUsize::new(2).unwrap());
    admission.register_operation(operation).unwrap();

    let index_one = fixture.open(operation_id, StreamDirection::SourceToDestination, 1, 0x40);
    fixture.admit(&mut admission, &index_one, 15).unwrap();

    let duplicate_id = fixture.open(operation_id, StreamDirection::SourceToDestination, 0, 0x40);
    assert_eq!(
        fixture.admit(&mut admission, &duplicate_id, 15),
        Err(StreamAdmissionError::DuplicateStreamId)
    );

    let duplicate_index = fixture.open(operation_id, StreamDirection::SourceToDestination, 1, 0x41);
    assert_eq!(
        fixture.admit(&mut admission, &duplicate_index, 15),
        Err(StreamAdmissionError::DuplicateStreamIndex)
    );

    let index_zero = fixture.open(operation_id, StreamDirection::SourceToDestination, 0, 0x42);
    fixture.admit(&mut admission, &index_zero, 15).unwrap();

    let out_of_range = fixture.open(operation_id, StreamDirection::SourceToDestination, 2, 0x43);
    assert_eq!(
        fixture.admit(&mut admission, &out_of_range, 15),
        Err(StreamAdmissionError::InvalidStreamIndex)
    );
}
