use crosslab_core::{
    ChannelBinding, LogicalSession, SessionActivation, SessionAuthRole, SessionAuthTranscriptV1,
    SessionError, SessionHandshakeSide, SessionState, TransportSecurityClass,
};
use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId, TrustRecord, TrustTransition};
use crosslab_protocol::{FeatureSet, ProtocolRange, ProtocolVersion};

fn establish_trust(
    credential: &DeviceCredential,
    authority: &OwnerAuthorityState,
    issuer_key: &SigningKey,
    transition_byte: u8,
) -> TrustRecord {
    let initial_credential = DeviceCredential::issue_for_public_key(
        credential.owner_id(),
        credential.device_id(),
        credential.device_public_key(),
        0,
        authority,
        issuer_key,
    )
    .unwrap();
    let transition = PairingTrustTransition::issue(
        &initial_credential,
        TransitionId::from_bytes([transition_byte; 32]),
        [transition_byte.wrapping_add(1); 32],
        authority,
        issuer_key,
    )
    .unwrap();
    let mut trust = transition
        .establish(&initial_credential, authority)
        .unwrap();

    for epoch in 1..=credential.credential_epoch() {
        let successor = DeviceCredential::issue_for_public_key(
            credential.owner_id(),
            credential.device_id(),
            credential.device_public_key(),
            epoch,
            authority,
            issuer_key,
        )
        .unwrap();
        let mut transition_id = [transition_byte; 32];
        transition_id[..8].copy_from_slice(&epoch.to_be_bytes());
        trust
            .accept_successor_credential(
                &successor,
                authority,
                TransitionId::from_bytes(transition_id),
            )
            .unwrap();
    }

    trust
}

struct Fixture {
    owner_id: OwnerId,
    root_key: SigningKey,
    authority: OwnerAuthorityState,
    issuer_key: SigningKey,
    initiator_key: SigningKey,
    responder_key: SigningKey,
    initiator_credential: DeviceCredential,
    responder_credential: DeviceCredential,
    responder_trust: TrustRecord,
}

impl Fixture {
    fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x90; 32]);
        let root_key = SigningKey::from_secret_bytes([0x91; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x92; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let initiator_key = SigningKey::from_secret_bytes([0x93; 32]);
        let responder_key = SigningKey::from_secret_bytes([0x94; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x95; 32]),
            &initiator_key,
            2,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x96; 32]),
            &responder_key,
            5,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let responder_trust = establish_trust(&responder_credential, &authority, &issuer_key, 0x97);

        Self {
            owner_id,
            root_key,
            authority,
            issuer_key,
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
            responder_trust,
        }
    }

    fn active_session(&self) -> LogicalSession {
        let ranges = [ProtocolRange::new(1, 0, 0).unwrap()];
        let features = FeatureSet::new(&[], &[]).unwrap();
        let binding = ChannelBinding::new("in-process-test", vec![0x98; 32]);
        let initiator_nonce = [0x99; 32];
        let responder_nonce = [0x9a; 32];
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
        let initiator = SessionHandshakeSide::new(&self.initiator_credential, &ranges, &features);
        let responder = SessionHandshakeSide::new(&self.responder_credential, &ranges, &features);
        let mut session = LogicalSession::new();
        session
            .authenticate(SessionActivation::new(
                &self.authority,
                initiator,
                responder,
                SessionAuthRole::Initiator,
                &self.responder_trust,
                initiator_nonce,
                responder_nonce,
                &binding,
                TransportSecurityClass::InProcessTest,
                &initiator_proof,
                &responder_proof,
            ))
            .unwrap();
        session
    }

    fn revoke(&self, record: TrustRecord, transition_byte: u8) -> TrustRecord {
        let transition = TrustTransition::issue_root_revocation(
            &record,
            TransitionId::from_bytes([transition_byte; 32]),
            &self.authority,
            &self.root_key,
        )
        .unwrap();
        let mut revoked = record;
        transition
            .apply_root(&mut revoked, &self.authority)
            .unwrap();
        revoked
    }
}

#[test]
fn transport_loss_closes_active_session_and_is_idempotent() {
    let fixture = Fixture::new();
    let mut session = fixture.active_session();

    session.transport_lost().unwrap();
    assert_eq!(session.state(), SessionState::Closed);
    assert!(session.context().is_some());

    session.transport_lost().unwrap();
    assert_eq!(session.state(), SessionState::Closed);

    let mut created = LogicalSession::new();
    assert_eq!(created.transport_lost(), Err(SessionError::InvalidState));
    assert_eq!(created.state(), SessionState::Created);
}

#[test]
fn accepted_matching_peer_revocation_takes_revoked_terminal_path() {
    let fixture = Fixture::new();
    let revoked = fixture.revoke(fixture.responder_trust, 0xa0);
    let mut session = fixture.active_session();

    session.apply_peer_revocation(&revoked).unwrap();
    assert_eq!(session.state(), SessionState::Revoked);

    session.finish_close().unwrap();
    assert_eq!(session.state(), SessionState::Closed);
}

#[test]
fn peer_revocation_rejects_unrevoked_mismatched_or_wrong_epoch_record() {
    let fixture = Fixture::new();

    let mut active = fixture.active_session();
    assert_eq!(
        active.apply_peer_revocation(&fixture.responder_trust),
        Err(SessionError::PeerNotRevoked)
    );
    assert_eq!(active.state(), SessionState::Active);

    let wrong_device_credential = DeviceCredential::issue(
        fixture.owner_id,
        DeviceId::from_bytes([0xa1; 32]),
        &fixture.responder_key,
        fixture.responder_credential.credential_epoch(),
        &fixture.authority,
        &fixture.issuer_key,
    )
    .unwrap();
    let wrong_device = establish_trust(
        &wrong_device_credential,
        &fixture.authority,
        &fixture.issuer_key,
        0xa2,
    );
    let wrong_device = fixture.revoke(wrong_device, 0xa3);
    assert_eq!(
        active.apply_peer_revocation(&wrong_device),
        Err(SessionError::PeerTrustMismatch)
    );
    assert_eq!(active.state(), SessionState::Active);

    let wrong_epoch_credential = DeviceCredential::issue(
        fixture.owner_id,
        fixture.responder_credential.device_id(),
        &fixture.responder_key,
        fixture.responder_credential.credential_epoch() + 1,
        &fixture.authority,
        &fixture.issuer_key,
    )
    .unwrap();
    let wrong_epoch = establish_trust(
        &wrong_epoch_credential,
        &fixture.authority,
        &fixture.issuer_key,
        0xa4,
    );
    let wrong_epoch = fixture.revoke(wrong_epoch, 0xa5);
    assert_eq!(
        active.apply_peer_revocation(&wrong_epoch),
        Err(SessionError::PeerCredentialEpochMismatch)
    );
    assert_eq!(active.state(), SessionState::Active);
}
