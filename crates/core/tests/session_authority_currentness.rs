use crosslab_core::{
    ChannelBinding, LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionError, SessionHandshakeSide, SessionState, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError,
    OwnerAuthorityState, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId};
use crosslab_protocol::{FeatureSet, ProtocolRange, ProtocolVersion};

#[test]
fn fresh_auth_rejects_credentials_from_superseded_device_signing_authority() {
    let owner_id = OwnerId::from_bytes([0x10; 32]);
    let root_key = SigningKey::from_secret_bytes([0x11; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer0_key = SigningKey::from_secret_bytes([0x12; 32]);
    let issuer0 = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer0_key,
        0,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(issuer0).unwrap();

    let initiator_key = SigningKey::from_secret_bytes([0x13; 32]);
    let responder_key = SigningKey::from_secret_bytes([0x14; 32]);
    let initiator_credential = DeviceCredential::issue_current(
        owner_id,
        DeviceId::from_bytes([0x15; 32]),
        &initiator_key,
        0,
        &authority,
        &issuer0_key,
    )
    .unwrap();
    let responder_credential = DeviceCredential::issue_current(
        owner_id,
        DeviceId::from_bytes([0x16; 32]),
        &responder_key,
        0,
        &authority,
        &issuer0_key,
    )
    .unwrap();
    let pairing = PairingTrustTransition::issue_current(
        &responder_credential,
        TransitionId::from_bytes([0x17; 32]),
        [0x18; 32],
        &authority,
        &issuer0_key,
    )
    .unwrap();
    let peer_trust = pairing
        .establish_current(&responder_credential, &authority)
        .unwrap();

    let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
    let features = FeatureSet::new(&[], &[]).unwrap();
    let binding = ChannelBinding::new("in-process-test", vec![0x19; 32]);
    let initiator_nonce = [0x1a; 32];
    let responder_nonce = [0x1b; 32];
    let transcript = SessionAuthTranscriptV1::new(
        owner_id,
        &initiator_credential,
        initiator_nonce,
        &responder_credential,
        responder_nonce,
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

    let issuer1_key = SigningKey::from_secret_bytes([0x1c; 32]);
    let issuer1 = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer1_key,
        1,
        &root_key,
    );
    authority.accept_delegation(issuer1).unwrap();

    let mut session = LogicalSession::new();
    let result = session.authenticate(SessionActivation::new(
        &authority,
        SessionHandshakeSide::new(&initiator_credential, &ranges, &features),
        SessionHandshakeSide::new(&responder_credential, &ranges, &features),
        SessionAuthRole::Initiator,
        &peer_trust,
        initiator_nonce,
        responder_nonce,
        &binding,
        TransportSecurityClass::InProcessTest,
        &initiator_proof,
        &responder_proof,
    ));

    assert_eq!(
        result,
        Err(SessionError::Identity(IdentityError::UnknownIssuer))
    );
    assert_eq!(session.state(), SessionState::Closed);
}
