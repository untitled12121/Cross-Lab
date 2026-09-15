use crosslab_core::{
    ChannelBinding, LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionError, SessionHandshakeSide, SessionState, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError,
    OwnerAuthorityState, OwnerId, OwnerRootRecord, RootSuccessor,
};
use crosslab_policy::{PairingTrustTransition, TransitionId, TrustRecord};
use crosslab_protocol::{FeatureSet, ProtocolRange, ProtocolVersion};

struct SessionFixture {
    owner_id: OwnerId,
    root_key: SigningKey,
    authority: OwnerAuthorityState,
    initiator_key: SigningKey,
    responder_key: SigningKey,
    initiator_credential: DeviceCredential,
    responder_credential: DeviceCredential,
    peer_trust: TrustRecord,
}

impl SessionFixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x10; 32]);
        let root_key = SigningKey::from_secret_bytes([0x11; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x12; 32]);
        let issuer = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(issuer).unwrap();

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

        Self {
            owner_id,
            root_key,
            authority,
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
            peer_trust,
        }
    }

    fn attempt_authenticate(&self) -> (LogicalSession, Result<(), SessionError>) {
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = ChannelBinding::new("in-process-test", vec![0x19; 32]);
        let initiator_nonce = [0x1a; 32];
        let responder_nonce = [0x1b; 32];
        let transcript = SessionAuthTranscriptV1::new(
            self.owner_id,
            &self.initiator_credential,
            initiator_nonce,
            &self.responder_credential,
            responder_nonce,
            ProtocolVersion::new(1, 0),
            &[],
            binding.profile_id().as_bytes(),
            binding.bytes(),
        )
        .unwrap();
        let initiator_proof = transcript
            .create_proof(SessionAuthRole::Initiator, &self.initiator_key)
            .unwrap();
        let responder_proof = transcript
            .create_proof(SessionAuthRole::Responder, &self.responder_key)
            .unwrap();

        let mut session = LogicalSession::new();
        let result = session.authenticate(SessionActivation::new(
            &self.authority,
            SessionHandshakeSide::new(&self.initiator_credential, &ranges, &features),
            SessionHandshakeSide::new(&self.responder_credential, &ranges, &features),
            SessionAuthRole::Initiator,
            &self.peer_trust,
            initiator_nonce,
            responder_nonce,
            &binding,
            TransportSecurityClass::InProcessTest,
            &initiator_proof,
            &responder_proof,
        ));
        (session, result)
    }

    fn rotate_device_signing(&mut self) {
        let next_key = SigningKey::from_secret_bytes([0x1c; 32]);
        let next = AuthorityDelegation::issue(
            self.owner_id,
            AuthorityRole::DeviceSigning,
            &next_key,
            1,
            &self.root_key,
        );
        self.authority.accept_delegation(next).unwrap();
    }

    fn accept_root_successor(&mut self) {
        let next_root_key = SigningKey::from_secret_bytes([0x1d; 32]);
        let successor =
            RootSuccessor::issue(self.authority.root(), &self.root_key, &next_root_key).unwrap();
        self.authority.accept_root_successor(&successor).unwrap();
    }
}

#[test]
fn fresh_auth_rejects_credentials_from_superseded_device_signing_authority() {
    let mut fixture = SessionFixture::new();
    fixture.rotate_device_signing();

    let (session, result) = fixture.attempt_authenticate();

    assert_eq!(
        result,
        Err(SessionError::Identity(IdentityError::UnknownIssuer))
    );
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn fresh_auth_rejects_after_root_successor_until_device_signing_is_reestablished() {
    let mut fixture = SessionFixture::new();
    fixture.accept_root_successor();

    let (session, result) = fixture.attempt_authenticate();

    assert_eq!(
        result,
        Err(SessionError::Identity(IdentityError::UnknownIssuer))
    );
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn active_session_snapshots_and_rejects_device_signing_rotation() {
    let mut fixture = SessionFixture::new();
    let root_key_id = fixture.authority.root().root_key_id();
    let root_epoch = fixture.authority.root().root_epoch();
    let dsa = *fixture
        .authority
        .current_delegation(AuthorityRole::DeviceSigning)
        .unwrap();
    let (mut session, result) = fixture.attempt_authenticate();
    result.unwrap();

    let context = session.context().unwrap();
    assert_eq!(context.root_key_id(), root_key_id);
    assert_eq!(context.root_epoch(), root_epoch);
    assert_eq!(context.device_signing_key_id(), dsa.delegated_key_id());
    assert_eq!(context.device_signing_epoch(), dsa.delegation_epoch());

    fixture.rotate_device_signing();

    assert_eq!(
        session.revalidate_authority(&fixture.authority),
        Err(SessionError::DeviceSigningAuthorityChanged)
    );
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn active_session_rejects_root_successor() {
    let mut fixture = SessionFixture::new();
    let (mut session, result) = fixture.attempt_authenticate();
    result.unwrap();

    fixture.accept_root_successor();

    assert_eq!(
        session.revalidate_authority(&fixture.authority),
        Err(SessionError::OwnerAuthorityChanged)
    );
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn administrative_and_recovery_rotation_do_not_invalidate_active_session() {
    let mut fixture = SessionFixture::new();
    let (mut session, result) = fixture.attempt_authenticate();
    result.unwrap();

    for (role, seed) in [
        (AuthorityRole::Administrative, 0x20),
        (AuthorityRole::Recovery, 0x30),
    ] {
        for epoch in 0..=1 {
            let key = SigningKey::from_secret_bytes([seed + epoch as u8; 32]);
            let delegation =
                AuthorityDelegation::issue(fixture.owner_id, role, &key, epoch, &fixture.root_key);
            fixture.authority.accept_delegation(delegation).unwrap();

            assert_eq!(session.revalidate_authority(&fixture.authority), Ok(()));
            assert_eq!(session.state(), SessionState::Active);
        }
    }
}
