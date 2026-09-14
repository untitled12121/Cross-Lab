use std::num::NonZeroUsize;

use crosslab_core::{
    ChannelBinding, ControlDispatchError, ControlDispatcher, EventSubscription, InboundControl,
    LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionHandshakeSide, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    ApprovalInstant, CapabilityId, CapabilityVersion, CapabilityVersionRange, LocalCapability,
    NetworkClass, PairingTrustTransition, PolicyState, TransitionId, TrustRecord,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlEnvelope, EnvelopeBody, Event,
    EventId, EventType, FeatureSet, ProtocolRange, ProtocolVersion,
};

const CAPABILITY: &str = "clipboard.write";
const EVENT_TYPE: &str = "clipboard.changed";

struct Fixture {
    root: OwnerRootRecord,
    delegation: AuthorityDelegation,
    local_key: SigningKey,
    peer_key: SigningKey,
    local_credential: DeviceCredential,
    peer_credential: DeviceCredential,
    peer_trust: TrustRecord,
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
        let local_key = SigningKey::from_secret_bytes([0x43; 32]);
        let peer_key = SigningKey::from_secret_bytes([0x44; 32]);
        let local_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x45; 32]),
            &local_key,
            0,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let peer_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x46; 32]),
            &peer_key,
            0,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let transition = PairingTrustTransition::issue(
            &peer_credential,
            TransitionId::from_bytes([0x47; 32]),
            [0x48; 32],
            &root,
            &delegation,
            &issuer_key,
            delegation.delegation_epoch(),
        )
        .unwrap();
        let peer_trust = transition
            .establish(
                &peer_credential,
                &root,
                &delegation,
                delegation.delegation_epoch(),
            )
            .unwrap();

        Self {
            root,
            delegation,
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
        let binding = ChannelBinding::new("in-process-test", vec![0x49; 32]);
        let transcript = SessionAuthTranscriptV1::new(
            self.root.owner_id(),
            &self.local_credential,
            [0x4a; 32],
            &self.peer_credential,
            [0x4b; 32],
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
                &self.root,
                SessionHandshakeSide::new(
                    &self.local_credential,
                    &self.delegation,
                    &ranges,
                    &features,
                ),
                SessionHandshakeSide::new(
                    &self.peer_credential,
                    &self.delegation,
                    &ranges,
                    &features,
                ),
                SessionAuthRole::Initiator,
                &self.peer_trust,
                [0x4a; 32],
                [0x4b; 32],
                &binding,
                TransportSecurityClass::InProcessTest,
                &local_proof,
                &peer_proof,
            ))
            .unwrap();
        session
            .negotiate_capabilities(
                &Self::local_capabilities(),
                &CapabilityAdvertisement::new(vec![
                    CapabilityAdvertisementEntry::new(
                        CapabilityId::parse(CAPABILITY).unwrap(),
                        CapabilityVersion::new(1, 0),
                        CapabilityVersion::new(1, 0),
                        true,
                    )
                    .unwrap(),
                ])
                .unwrap(),
            )
            .unwrap();
        session
    }
}

fn capability_event(event_type: &str) -> Event {
    Event::capability(
        EventId::from_bytes([0x50; 16]),
        CapabilityId::parse(CAPABILITY).unwrap(),
        EventType::parse(event_type).unwrap(),
        vec![0x51],
    )
    .unwrap()
}

fn accept_event(
    dispatcher: &mut ControlDispatcher,
    session: &LogicalSession,
    fixture: &Fixture,
    event: Event,
) -> Result<InboundControl, ControlDispatchError> {
    let context = session.context().unwrap();
    dispatcher.accept_inbound(
        context,
        ControlEnvelope::new(
            context.protocol_version(),
            context.session_id(),
            0,
            EnvelopeBody::Event(event),
        ),
        &PolicyState::new(),
        &Fixture::local_capabilities(),
        &fixture.peer_trust,
        NetworkClass::Local,
        ApprovalInstant::from_ticks(0),
    )
}

#[test]
fn negotiated_capability_does_not_authorize_unsubscribed_event() {
    let fixture = Fixture::new();
    let session = fixture.session();
    let mut dispatcher =
        ControlDispatcher::new(session.context().unwrap(), NonZeroUsize::new(4).unwrap());

    assert_eq!(
        accept_event(
            &mut dispatcher,
            &session,
            &fixture,
            capability_event(EVENT_TYPE),
        ),
        Err(ControlDispatchError::EventNotSubscribed)
    );
}

#[test]
fn subscription_is_exact_to_capability_and_event_type() {
    let fixture = Fixture::new();
    let session = fixture.session();
    let mut dispatcher =
        ControlDispatcher::new(session.context().unwrap(), NonZeroUsize::new(4).unwrap());
    dispatcher.subscribe_event(EventSubscription::new(
        CapabilityId::parse(CAPABILITY).unwrap(),
        EventType::parse("clipboard.other").unwrap(),
    ));

    assert_eq!(
        accept_event(
            &mut dispatcher,
            &session,
            &fixture,
            capability_event(EVENT_TYPE),
        ),
        Err(ControlDispatchError::EventNotSubscribed)
    );
}

#[test]
fn exact_local_subscription_authorizes_capability_event() {
    let fixture = Fixture::new();
    let session = fixture.session();
    let mut dispatcher =
        ControlDispatcher::new(session.context().unwrap(), NonZeroUsize::new(4).unwrap());
    dispatcher.subscribe_event(EventSubscription::new(
        CapabilityId::parse(CAPABILITY).unwrap(),
        EventType::parse(EVENT_TYPE).unwrap(),
    ));

    assert!(matches!(
        accept_event(
            &mut dispatcher,
            &session,
            &fixture,
            capability_event(EVENT_TYPE),
        ),
        Ok(InboundControl::Event(_))
    ));
}

#[test]
fn authenticated_system_event_does_not_require_capability_subscription() {
    let fixture = Fixture::new();
    let session = fixture.session();
    let mut dispatcher =
        ControlDispatcher::new(session.context().unwrap(), NonZeroUsize::new(4).unwrap());
    let event = Event::system(
        EventId::from_bytes([0x52; 16]),
        EventType::parse("crosslab.system.test").unwrap(),
        Vec::new(),
    )
    .unwrap();

    assert!(matches!(
        accept_event(&mut dispatcher, &session, &fixture, event),
        Ok(InboundControl::Event(_))
    ));
}
