use std::num::{NonZeroU32, NonZeroUsize};

use crosslab_core::{
    LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionHandshakeSide, StreamAdmissionError, StreamReceiveError, TransportConnection,
    TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    AuthorizationContext, AuthorizedOperation, CapabilityId, CapabilityVersion,
    CapabilityVersionRange, LocalCapability, NetworkClass, OperationId, OperationName, PolicyRule,
    PolicyState, RuleEffect, RuleId, TransitionId, TrustRecord, TrustState, UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, DataStreamOpen, FeatureSet,
    ProtocolRange, ProtocolVersion, StreamDirection, StreamId,
};
use crosslab_sim::{
    stream::{SimStreamError, SimStreamRuntime},
    transport::MemoryTransportPair,
};

struct Fixture {
    sender_session: LogicalSession,
    receiver_session: LogicalSession,
    capability: CapabilityId,
    version: CapabilityVersion,
    operation_name: OperationName,
    trust_revision: u64,
    policy_revision: u64,
    grant: crosslab_policy::AuthorizationGrant,
}

impl Fixture {
    fn new(pair: &MemoryTransportPair) -> Self {
        let owner_id = OwnerId::from_bytes([0x60; 32]);
        let root_key = SigningKey::from_secret_bytes([0x61; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x62; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let sender_key = SigningKey::from_secret_bytes([0x63; 32]);
        let receiver_key = SigningKey::from_secret_bytes([0x64; 32]);
        let sender_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x65; 32]),
            &sender_key,
            1,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let receiver_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x66; 32]),
            &receiver_key,
            1,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let sender_trust = TrustRecord::trusted(
            owner_id,
            sender_credential.device_id(),
            sender_credential.credential_epoch(),
            TransitionId::from_bytes([0x67; 32]),
        );
        let receiver_trust = TrustRecord::trusted(
            owner_id,
            receiver_credential.device_id(),
            receiver_credential.credential_epoch(),
            TransitionId::from_bytes([0x68; 32]),
        );
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = pair.endpoints().0.channel_binding();
        let transcript = SessionAuthTranscriptV1::new(
            owner_id,
            &sender_credential,
            [0x69; 32],
            &receiver_credential,
            [0x6a; 32],
            ProtocolVersion::new(1, 0),
            &[],
            binding.profile_id().as_bytes(),
            binding.bytes(),
        )
        .unwrap();
        let sender_proof = transcript
            .create_proof(SessionAuthRole::Initiator, &sender_key)
            .unwrap();
        let receiver_proof = transcript
            .create_proof(SessionAuthRole::Responder, &receiver_key)
            .unwrap();
        let sender_side =
            SessionHandshakeSide::new(&sender_credential, &delegation, &ranges, &features);
        let receiver_side =
            SessionHandshakeSide::new(&receiver_credential, &delegation, &ranges, &features);

        let mut sender_session = LogicalSession::new();
        sender_session
            .authenticate(SessionActivation::new(
                &root,
                sender_side,
                receiver_side,
                SessionAuthRole::Initiator,
                &receiver_trust,
                [0x69; 32],
                [0x6a; 32],
                binding,
                TransportSecurityClass::InProcessTest,
                &sender_proof,
                &receiver_proof,
            ))
            .unwrap();
        let mut receiver_session = LogicalSession::new();
        receiver_session
            .authenticate(SessionActivation::new(
                &root,
                sender_side,
                receiver_side,
                SessionAuthRole::Responder,
                &sender_trust,
                [0x69; 32],
                [0x6a; 32],
                binding,
                TransportSecurityClass::InProcessTest,
                &sender_proof,
                &receiver_proof,
            ))
            .unwrap();

        let capability = CapabilityId::parse("files.transfer").unwrap();
        let version = CapabilityVersion::new(1, 0);
        let operation_name = OperationName::parse("send").unwrap();
        let local_capability = LocalCapability::new(
            capability.clone(),
            CapabilityVersionRange::new(1, 0, 0).unwrap(),
            true,
        );
        let advertisement = CapabilityAdvertisement::new(vec![
            CapabilityAdvertisementEntry::new(capability.clone(), version, version, true).unwrap(),
        ])
        .unwrap();
        for session in [&mut sender_session, &mut receiver_session] {
            session
                .negotiate_capabilities(std::slice::from_ref(&local_capability), &advertisement)
                .unwrap();
        }

        let trust_revision = sender_trust.trust_revision();
        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([0x6b; 32]),
                sender_credential.device_id(),
                capability.clone(),
                operation_name.clone(),
                RuleEffect::Allow,
            ))
            .unwrap();
        let context = AuthorizationContext::new(
            sender_credential.device_id(),
            receiver_credential.device_id(),
            receiver_session.context().unwrap().session_id(),
            capability.clone(),
            version,
            operation_name.clone(),
            TrustState::Trusted,
            trust_revision,
            local_capability,
            NetworkClass::Local,
        );
        let grant = policy.evaluate(&context).into_grant().unwrap();

        Self {
            sender_session,
            receiver_session,
            capability,
            version,
            operation_name,
            trust_revision,
            policy_revision: policy.revision(),
            grant,
        }
    }

    fn operation(&self, use_policy: UsePolicy) -> AuthorizedOperation {
        AuthorizedOperation::issue(self.grant.clone(), 10, 20, use_policy).unwrap()
    }

    fn open(&self, operation_id: OperationId, stream_index: u32, stream_byte: u8) -> DataStreamOpen {
        DataStreamOpen::new(
            self.sender_session.context().unwrap().session_id(),
            StreamId::from_bytes([stream_byte; 16]),
            operation_id,
            self.capability.clone(),
            self.version,
            self.operation_name.clone(),
            StreamDirection::SourceToDestination,
            stream_index,
        )
    }
}

fn transport_pair() -> MemoryTransportPair {
    MemoryTransportPair::new(NonZeroUsize::new(8).unwrap(), [0x70; 32])
}

#[test]
fn s007_authorized_single_stream_flows_in_order_and_cannot_be_reused() {
    let pair = transport_pair();
    let fixture = Fixture::new(&pair);
    let (sender_endpoint, receiver_endpoint) = pair.endpoints();
    let operation = fixture.operation(UsePolicy::SingleStream);
    let operation_id = operation.id();
    let open = fixture.open(operation_id, 0, 0x71);
    let sender = SimStreamRuntime::new(
        &fixture.sender_session,
        sender_endpoint,
        NonZeroUsize::new(4).unwrap(),
    )
    .unwrap();
    let mut receiver = SimStreamRuntime::new(
        &fixture.receiver_session,
        receiver_endpoint,
        NonZeroUsize::new(4).unwrap(),
    )
    .unwrap();
    receiver.register_operation(operation).unwrap();

    let mut send = sender.open_uni(&open).unwrap();
    let stream_id = receiver
        .accept_one(15, fixture.trust_revision, fixture.policy_revision)
        .unwrap();
    assert_eq!(stream_id, open.stream_id());

    for chunk in [vec![1], vec![2], vec![3]] {
        send.try_send_chunk(chunk).unwrap();
    }
    assert_eq!(receiver.try_receive_chunk(stream_id).unwrap(), vec![1]);
    assert_eq!(receiver.try_receive_chunk(stream_id).unwrap(), vec![2]);
    assert_eq!(receiver.try_receive_chunk(stream_id).unwrap(), vec![3]);

    send.finish();
    assert!(matches!(
        receiver.try_receive_chunk(stream_id),
        Err(SimStreamError::Receive(StreamReceiveError::Finished))
    ));

    let second = fixture.open(operation_id, 0, 0x72);
    let mut second_send = sender.open_uni(&second).unwrap();
    assert!(matches!(
        receiver.accept_one(15, fixture.trust_revision, fixture.policy_revision),
        Err(SimStreamError::Admission(
            StreamAdmissionError::DuplicateStreamIndex
        ))
    ));
    assert!(second_send.try_send_chunk(vec![9]).is_err());
}

#[test]
fn saturated_runtime_leaves_pending_stream_and_operation_budget_unspent() {
    let pair = transport_pair();
    let fixture = Fixture::new(&pair);
    let (sender_endpoint, receiver_endpoint) = pair.endpoints();
    let multi = fixture.operation(UsePolicy::MultiStream {
        max_streams: NonZeroU32::new(2).unwrap(),
    });
    let single = fixture.operation(UsePolicy::SingleStream);
    let multi_id = multi.id();
    let single_id = single.id();
    let multi_first = fixture.open(multi_id, 0, 0x73);
    let single_open = fixture.open(single_id, 0, 0x74);
    let multi_second = fixture.open(multi_id, 1, 0x75);
    let sender = SimStreamRuntime::new(
        &fixture.sender_session,
        sender_endpoint,
        NonZeroUsize::new(2).unwrap(),
    )
    .unwrap();
    let mut receiver = SimStreamRuntime::new(
        &fixture.receiver_session,
        receiver_endpoint,
        NonZeroUsize::new(2).unwrap(),
    )
    .unwrap();
    receiver.register_operation(multi).unwrap();
    receiver.register_operation(single).unwrap();

    let _multi_first_send = sender.open_uni(&multi_first).unwrap();
    let mut single_send = sender.open_uni(&single_open).unwrap();
    let first_stream_id = receiver
        .accept_one(15, fixture.trust_revision, fixture.policy_revision)
        .unwrap();
    let single_stream_id = receiver
        .accept_one(15, fixture.trust_revision, fixture.policy_revision)
        .unwrap();
    assert_eq!(first_stream_id, multi_first.stream_id());
    assert_eq!(single_stream_id, single_open.stream_id());

    let _multi_second_send = sender.open_uni(&multi_second).unwrap();
    assert!(matches!(
        receiver.accept_one(15, fixture.trust_revision, fixture.policy_revision),
        Err(SimStreamError::ResourceLimit)
    ));

    single_send.finish();
    assert!(matches!(
        receiver.try_receive_chunk(single_stream_id),
        Err(SimStreamError::Receive(StreamReceiveError::Finished))
    ));

    assert_eq!(
        receiver
            .accept_one(15, fixture.trust_revision, fixture.policy_revision)
            .unwrap(),
        multi_second.stream_id()
    );
}
