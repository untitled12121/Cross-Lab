use std::num::NonZeroUsize;

use crosslab_core::{
    ChannelBinding, ControlDispatchError, ControlDispatcher, LogicalSession, SessionActivation,
    SessionAuthRole, SessionAuthTranscriptV1, SessionHandshakeSide, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{NetworkClass, PairingTrustTransition, PolicyState, TransitionId};
use crosslab_protocol::{
    CapabilityAdvertisement, ControlEnvelope, EnvelopeBody, FeatureSet, ProtocolRange,
    ProtocolVersion,
};

#[test]
fn rotated_peer_credential_invalidates_old_control_snapshot() {
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
    let peer_old_key = SigningKey::from_secret_bytes([0x44; 32]);
    let peer_new_key = SigningKey::from_secret_bytes([0x45; 32]);
    let local_device_id = DeviceId::from_bytes([0x46; 32]);
    let peer_device_id = DeviceId::from_bytes([0x47; 32]);
    let local_credential = DeviceCredential::issue(
        owner_id,
        local_device_id,
        &local_key,
        0,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();
    let peer_credential = DeviceCredential::issue(
        owner_id,
        peer_device_id,
        &peer_old_key,
        0,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();
    let transition = PairingTrustTransition::issue(
        &peer_credential,
        TransitionId::from_bytes([0x48; 32]),
        [0x4d; 32],
        &root,
        &delegation,
        &issuer_key,
        delegation.delegation_epoch(),
    )
    .unwrap();
    let mut peer_trust = transition
        .establish(
            &peer_credential,
            &root,
            &delegation,
            delegation.delegation_epoch(),
        )
        .unwrap();
    let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
    let features = FeatureSet::new(&[], &[]).unwrap();
    let binding = ChannelBinding::new("in-process-test", vec![0x49; 32]);
    let transcript = SessionAuthTranscriptV1::new(
        owner_id,
        &local_credential,
        [0x4a; 32],
        &peer_credential,
        [0x4b; 32],
        ProtocolVersion::new(1, 0),
        &[],
        binding.profile_id().as_bytes(),
        binding.bytes(),
    )
    .unwrap();
    let initiator_proof = transcript
        .create_proof(SessionAuthRole::Initiator, &local_key)
        .unwrap();
    let responder_proof = transcript
        .create_proof(SessionAuthRole::Responder, &peer_old_key)
        .unwrap();
    let mut session = LogicalSession::new();
    session
        .authenticate(SessionActivation::new(
            &root,
            SessionHandshakeSide::new(&local_credential, &delegation, &ranges, &features),
            SessionHandshakeSide::new(&peer_credential, &delegation, &ranges, &features),
            SessionAuthRole::Initiator,
            &peer_trust,
            [0x4a; 32],
            [0x4b; 32],
            &binding,
            TransportSecurityClass::InProcessTest,
            &initiator_proof,
            &responder_proof,
        ))
        .unwrap();

    let successor = DeviceCredential::issue(
        owner_id,
        peer_device_id,
        &peer_new_key,
        1,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();
    peer_trust
        .accept_successor_credential(
            &successor,
            &root,
            &delegation,
            delegation.delegation_epoch(),
            TransitionId::from_bytes([0x4c; 32]),
        )
        .unwrap();

    let context = session.context().unwrap();
    let mut dispatcher = ControlDispatcher::new(context, NonZeroUsize::new(4).unwrap());
    let envelope = ControlEnvelope::new(
        context.protocol_version(),
        context.session_id(),
        0,
        EnvelopeBody::CapabilityAdvertisement(CapabilityAdvertisement::new(Vec::new()).unwrap()),
    );

    assert_eq!(
        dispatcher.accept_inbound(
            context,
            envelope,
            &PolicyState::new(),
            &[],
            &peer_trust,
            NetworkClass::Local,
        ),
        Err(ControlDispatchError::PeerCredentialEpochChanged)
    );
}
