use crosslab_core::{
    PairingId, PairingTranscript, SessionAuthProof, SessionAuthRole, SessionAuthTranscriptV1,
};
use crosslab_crypto::{SigningKey, blake3_256};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_protocol::ProtocolVersion;

#[test]
fn session_auth_debug_redacts_proof_and_transcript_security_material() {
    let fixture = session_fixture();
    let initiator_nonce = [0xa1; 32];
    let responder_nonce = [0xb2; 32];
    let binding_profile = b"privacy-binding-profile-sentinel";
    let binding_value = [0xc3; 32];
    let transcript = SessionAuthTranscriptV1::new(
        fixture.owner_id,
        &fixture.initiator_credential,
        initiator_nonce,
        &fixture.responder_credential,
        responder_nonce,
        ProtocolVersion::new(1, 3),
        &[9, 1, 4],
        binding_profile,
        &binding_value,
    )
    .unwrap();
    let proof = transcript
        .create_proof(SessionAuthRole::Initiator, &fixture.initiator_key)
        .unwrap();

    let transcript_debug = format!("{transcript:?}");
    assert!(transcript_debug.contains("SessionAuthTranscriptV1"));
    assert_debug_omits(&transcript_debug, &initiator_nonce);
    assert_debug_omits(&transcript_debug, &responder_nonce);
    assert_debug_omits(
        &transcript_debug,
        &labeled_digest(
            b"crosslab.session-auth.channel-binding-profile.v1\0",
            binding_profile,
        ),
    );
    assert_debug_omits(
        &transcript_debug,
        &labeled_digest(
            b"crosslab.session-auth.channel-binding-value.v1\0",
            &binding_value,
        ),
    );

    assert_proof_debug_is_redacted(&proof);
}

#[test]
fn pairing_transcript_debug_redacts_bootstrap_security_material() {
    let inviter_key = SigningKey::from_secret_bytes([0xd1; 32]);
    let joiner_key = SigningKey::from_secret_bytes([0xd2; 32]);
    let pairing_id = PairingId::from_bytes([0xd3; 16]);
    let inviter_nonce = [0xd4; 32];
    let joiner_nonce = [0xd5; 32];
    let transcript = PairingTranscript::new(
        1,
        pairing_id,
        OwnerId::from_bytes([0xd6; 32]),
        DeviceId::from_bytes([0xd7; 32]),
        inviter_key.verifying_key(),
        inviter_nonce,
        DeviceId::from_bytes([0xd8; 32]),
        joiner_key.verifying_key(),
        joiner_nonce,
    );

    let debug = format!("{transcript:?}");
    assert!(debug.contains("PairingTranscript"));
    assert_debug_omits(&debug, pairing_id.as_bytes());
    assert_debug_omits(&debug, &inviter_nonce);
    assert_debug_omits(&debug, &joiner_nonce);
}

fn assert_proof_debug_is_redacted(proof: &SessionAuthProof) {
    let debug = format!("{proof:?}");
    assert!(debug.contains("SessionAuthProof"));
    assert!(debug.contains("Initiator"));
    assert_debug_omits(&debug, &proof.transcript_digest());
    assert_debug_omits(&debug, &proof.signature().to_bytes());
}

fn assert_debug_omits<const N: usize>(debug: &str, bytes: &[u8; N]) {
    let exposed = format!("{bytes:?}");
    assert!(
        !debug.contains(&exposed),
        "Debug output exposed protected bytes: {debug}"
    );
}

fn labeled_digest(label: &[u8], value: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(label.len() + value.len());
    input.extend_from_slice(label);
    input.extend_from_slice(value);
    blake3_256(&input)
}

struct SessionFixture {
    owner_id: OwnerId,
    initiator_key: SigningKey,
    initiator_credential: DeviceCredential,
    responder_credential: DeviceCredential,
}

fn session_fixture() -> SessionFixture {
    let owner_id = OwnerId::from_bytes([0xe1; 32]);
    let root_key = SigningKey::from_secret_bytes([0xe2; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let issuer_key = SigningKey::from_secret_bytes([0xe3; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        0,
        &root_key,
    );
    let initiator_key = SigningKey::from_secret_bytes([0xe4; 32]);
    let responder_key = SigningKey::from_secret_bytes([0xe5; 32]);
    let initiator_credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([0xe6; 32]),
        &initiator_key,
        1,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();
    let responder_credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([0xe7; 32]),
        &responder_key,
        1,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap();

    SessionFixture {
        owner_id,
        initiator_key,
        initiator_credential,
        responder_credential,
    }
}
