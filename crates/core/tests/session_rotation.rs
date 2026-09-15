use crosslab_core::{
    ChannelBinding, LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionError, SessionHandshakeSide, SessionState, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId};
use crosslab_protocol::{FeatureSet, ProtocolRange, ProtocolVersion};

#[test]
fn accepted_peer_rotation_invalidates_old_authenticated_session() {
    let owner_id = OwnerId::from_bytes([0x30; 32]);
    let root_key = SigningKey::from_secret_bytes([0x31; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer_key = SigningKey::from_secret_bytes([0x32; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        0,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();

    let local_key = SigningKey::from_secret_bytes([0x33; 32]);
    let peer_old_key = SigningKey::from_secret_bytes([0x34; 32]);
    let peer_new_key = SigningKey::from_secret_bytes([0x35; 32]);
    let local_device_id = DeviceId::from_bytes([0x36; 32]);
    let peer_device_id = DeviceId::from_bytes([0x37; 32]);
    let local_credential = DeviceCredential::issue(
        owner_id,
        local_device_id,
        &local_key,
        0,
        &authority,
        &issuer_key,
    )
    .unwrap();
    let peer_credential = DeviceCredential::issue(
        owner_id,
        peer_device_id,
        &peer_old_key,
        0,
        &authority,
        &issuer_key,
    )
    .unwrap();
    let transition = PairingTrustTransition::issue(
        &peer_credential,
        TransitionId::from_bytes([0x38; 32]),
        [0x3d; 32],
        &authority,
        &issuer_key,
    )
    .unwrap();
    let mut peer_trust = transition.establish(&peer_credential, &authority).unwrap();
    let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
    let features = FeatureSet::new(&[], &[]).unwrap();
    let binding = ChannelBinding::new("in-process-test", vec![0x39; 32]);
    let transcript = SessionAuthTranscriptV1::new(
        owner_id,
        &local_credential,
        [0x3a; 32],
        &peer_credential,
        [0x3b; 32],
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
            &authority,
            SessionHandshakeSide::new(&local_credential, &ranges, &features),
            SessionHandshakeSide::new(&peer_credential, &ranges, &features),
            SessionAuthRole::Initiator,
            &peer_trust,
            [0x3a; 32],
            [0x3b; 32],
            &binding,
            TransportSecurityClass::InProcessTest,
            &initiator_proof,
            &responder_proof,
        ))
        .unwrap();
    assert_eq!(session.state(), SessionState::Active);

    let successor = DeviceCredential::issue(
        owner_id,
        peer_device_id,
        &peer_new_key,
        1,
        &authority,
        &issuer_key,
    )
    .unwrap();
    peer_trust
        .accept_successor_credential(&successor, &authority, TransitionId::from_bytes([0x3c; 32]))
        .unwrap();

    assert_eq!(
        session.revalidate_peer_trust(&peer_trust),
        Err(SessionError::PeerCredentialEpochMismatch)
    );
    assert_eq!(session.state(), SessionState::Closed);
}
