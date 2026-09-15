use std::num::NonZeroUsize;

use crosslab_core::{
    ChannelBinding, ControlDispatchError, ControlDispatcher, InboundControl, LogicalSession,
    SessionActivation, SessionAuthRole, SessionAuthTranscriptV1, SessionHandshakeSide,
    TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{
    ApprovalInstant, CapabilityId, CapabilityVersion, CapabilityVersionRange, LocalCapability,
    NetworkClass, OperationName, PairingTrustTransition, PolicyRule, PolicyState, RuleEffect,
    RuleId, TransitionId, TrustRecord,
};
use crosslab_protocol::{
    CancelRequest, CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlEnvelope,
    ControlRequest, ControlResponse, ControlResponseResult, EnvelopeBody, FeatureSet,
    ProtocolRange, ProtocolVersion, RequestId, RetryClass,
};

const CAPABILITY: &str = "clipboard.write";
const OPERATION: &str = "set";

struct Fixture {
    authority: OwnerAuthorityState,
    local_key: SigningKey,
    peer_key: SigningKey,
    local_credential: DeviceCredential,
    peer_credential: DeviceCredential,
    peer_trust: TrustRecord,
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
        let local_key = SigningKey::from_secret_bytes([0x13; 32]);
        let peer_key = SigningKey::from_secret_bytes([0x14; 32]);
        let local_credential = DeviceCredential::issue_current(
            owner_id,
            DeviceId::from_bytes([0x15; 32]),
            &local_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let peer_credential = DeviceCredential::issue_current(
            owner_id,
            DeviceId::from_bytes([0x16; 32]),
            &peer_key,
            0,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let transition = PairingTrustTransition::issue_current(
            &peer_credential,
            TransitionId::from_bytes([0x17; 32]),
            [0x18; 32],
            &authority,
            &issuer_key,
        )
        .unwrap();
        let peer_trust = transition
            .establish_current(&peer_credential, &authority)
            .unwrap();

        Self {
            authority,
            local_key,
            peer_key,
            local_credential,
            peer_credential,
            peer_trust,
        }
    }

    fn local_capabilities() -> Vec<LocalCapability> {
        vec![LocalCapability::new(
            CapabilityId::parse(CAPABILITY).unwrap(),
            CapabilityVersionRange::new(1, 0, 0).unwrap(),
            true,
        )]
    }

    fn session(&self) -> LogicalSession {
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = ChannelBinding::new("in-process-test", vec![0x19; 32]);
        let transcript = SessionAuthTranscriptV1::new(
            self.authority.root().owner_id(),
            &self.local_credential,
            [0x1a; 32],
            &self.peer_credential,
            [0x1b; 32],
            ProtocolVersion::new(1, 0),
            &[],
            binding.profile_id().as_bytes(),
            binding.bytes(),
        )
        .unwrap();
        let local_proof = transcript
            .create_proof(SessionAuthRole::Initiator, &self.local_key)
            .unwrap();
        let peer_proof = transcript
            .create_proof(SessionAuthRole::Responder, &self.peer_key)
            .unwrap();
        let mut session = LogicalSession::new();
        session
            .authenticate(SessionActivation::new(
                &self.authority,
                SessionHandshakeSide::new(&self.local_credential, &ranges, &features),
                SessionHandshakeSide::new(&self.peer_credential, &ranges, &features),
                SessionAuthRole::Initiator,
                &self.peer_trust,
                [0x1a; 32],
                [0x1b; 32],
                &binding,
                TransportSecurityClass::InProcessTest,
                &local_proof,
                &peer_proof,
            ))
            .unwrap();
        let local_capabilities = Self::local_capabilities();
        let advertisement = CapabilityAdvertisement::new(vec![
            CapabilityAdvertisementEntry::new(
                CapabilityId::parse(CAPABILITY).unwrap(),
                CapabilityVersion::new(1, 0),
                CapabilityVersion::new(1, 0),
                true,
            )
            .unwrap(),
        ])
        .unwrap();
        session
            .negotiate_capabilities(&local_capabilities, &advertisement)
            .unwrap();
        session
    }

    fn allow_policy(&self) -> PolicyState {
        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([0x1c; 32]),
                self.peer_credential.device_id(),
                CapabilityId::parse(CAPABILITY).unwrap(),
                OperationName::parse(OPERATION).unwrap(),
                RuleEffect::Allow,
            ))
            .unwrap();
        policy
    }
}

fn request(id: RequestId, retry_class: RetryClass) -> ControlRequest {
    ControlRequest::new(
        id,
        CapabilityId::parse(CAPABILITY).unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse(OPERATION).unwrap(),
        retry_class,
        vec![0x20],
    )
}

fn accept_request(
    dispatcher: &mut ControlDispatcher,
    session: &LogicalSession,
    request: ControlRequest,
    sequence: u64,
    fixture: &Fixture,
    policy: &PolicyState,
) -> Result<InboundControl, ControlDispatchError> {
    let context = session.context().unwrap();
    dispatcher.accept_inbound(
        context,
        ControlEnvelope::new(
            context.protocol_version(),
            context.session_id(),
            sequence,
            EnvelopeBody::ControlRequest(request),
        ),
        policy,
        &Fixture::local_capabilities(),
        &fixture.peer_trust,
        NetworkClass::Local,
        ApprovalInstant::from_ticks(0),
    )
}

fn complete_request(dispatcher: &mut ControlDispatcher, session: &LogicalSession, id: RequestId) {
    let response = ControlResponse::new(id, ControlResponseResult::Success(Vec::new()));
    let envelope = dispatcher
        .prepare_outbound(
            session.context().unwrap(),
            EnvelopeBody::ControlResponse(response),
        )
        .unwrap();
    dispatcher.commit_outbound(&envelope);
}

#[test]
fn peer_declared_idempotence_does_not_authorize_duplicate_request_id() {
    let fixture = Fixture::new();
    let session = fixture.session();
    let policy = fixture.allow_policy();
    let mut dispatcher =
        ControlDispatcher::new(session.context().unwrap(), NonZeroUsize::new(4).unwrap());
    let id = RequestId::from_bytes([0x30; 16]);

    assert!(matches!(
        accept_request(
            &mut dispatcher,
            &session,
            request(id, RetryClass::Idempotent),
            0,
            &fixture,
            &policy,
        ),
        Ok(InboundControl::Request(_))
    ));
    complete_request(&mut dispatcher, &session, id);

    assert_eq!(
        accept_request(
            &mut dispatcher,
            &session,
            request(id, RetryClass::Idempotent),
            1,
            &fixture,
            &policy,
        ),
        Err(ControlDispatchError::DuplicateRequest)
    );
}

#[test]
fn completed_replay_state_is_bounded_without_permanent_exhaustion() {
    let fixture = Fixture::new();
    let session = fixture.session();
    let policy = fixture.allow_policy();
    let mut dispatcher =
        ControlDispatcher::new(session.context().unwrap(), NonZeroUsize::new(2).unwrap());

    for (sequence, byte) in [(0, 0x31), (1, 0x32)] {
        let id = RequestId::from_bytes([byte; 16]);
        assert!(matches!(
            accept_request(
                &mut dispatcher,
                &session,
                request(id, RetryClass::NonRetryable),
                sequence,
                &fixture,
                &policy,
            ),
            Ok(InboundControl::Request(_))
        ));
        complete_request(&mut dispatcher, &session, id);
    }

    assert!(matches!(
        accept_request(
            &mut dispatcher,
            &session,
            request(RequestId::from_bytes([0x33; 16]), RetryClass::NonRetryable),
            2,
            &fixture,
            &policy,
        ),
        Ok(InboundControl::Request(_))
    ));
}

#[test]
fn cancelled_request_id_remains_in_local_replay_window() {
    let fixture = Fixture::new();
    let session = fixture.session();
    let policy = fixture.allow_policy();
    let mut dispatcher =
        ControlDispatcher::new(session.context().unwrap(), NonZeroUsize::new(4).unwrap());
    let id = RequestId::from_bytes([0x34; 16]);

    assert!(matches!(
        accept_request(
            &mut dispatcher,
            &session,
            request(id, RetryClass::Idempotent),
            0,
            &fixture,
            &policy,
        ),
        Ok(InboundControl::Request(_))
    ));

    let context = session.context().unwrap();
    assert_eq!(
        dispatcher.accept_inbound(
            context,
            ControlEnvelope::new(
                context.protocol_version(),
                context.session_id(),
                1,
                EnvelopeBody::CancelRequest(CancelRequest::new(id)),
            ),
            &policy,
            &Fixture::local_capabilities(),
            &fixture.peer_trust,
            NetworkClass::Local,
            ApprovalInstant::from_ticks(0),
        ),
        Ok(InboundControl::Cancelled(id))
    );

    assert_eq!(
        accept_request(
            &mut dispatcher,
            &session,
            request(id, RetryClass::Idempotent),
            2,
            &fixture,
            &policy,
        ),
        Err(ControlDispatchError::DuplicateRequest)
    );
}
