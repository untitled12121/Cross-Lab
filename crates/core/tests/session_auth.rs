use crosslab_core::{SessionAuthError, SessionAuthRole, SessionAuthTranscriptV1};
use crosslab_crypto::{
    CanonicalTranscript, SignatureAlgorithm, SigningKey, blake3_256, signed_object_digest,
};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_protocol::ProtocolVersion;

const FEATURE_SET_DOMAIN: &[u8] = b"crosslab.session-auth.feature-set.v1\0";
const CHANNEL_PROFILE_DOMAIN: &[u8] = b"crosslab.session-auth.channel-binding-profile.v1\0";
const CHANNEL_VALUE_DOMAIN: &[u8] = b"crosslab.session-auth.channel-binding-value.v1\0";
const INITIATOR_PROOF_LABEL: &[u8] = b"crosslab.session-auth.initiator-proof.v1";

struct Fixture {
    owner_id: OwnerId,
    initiator_key: SigningKey,
    responder_key: SigningKey,
    initiator_credential: DeviceCredential,
    responder_credential: DeviceCredential,
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
        let initiator_key = SigningKey::from_secret_bytes([0x13; 32]);
        let responder_key = SigningKey::from_secret_bytes([0x14; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x15; 32]),
            &initiator_key,
            2,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x16; 32]),
            &responder_key,
            5,
            &root,
            &delegation,
            &issuer_key,
        )
        .unwrap();

        Self {
            owner_id,
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
        }
    }

    fn transcript(&self) -> SessionAuthTranscriptV1 {
        SessionAuthTranscriptV1::new(
            self.owner_id,
            &self.initiator_credential,
            [0x21; 32],
            &self.responder_credential,
            [0x22; 32],
            ProtocolVersion::new(1, 3),
            &[9, 1, 4, 4],
            b"in-process-test",
            &[0x23; 32],
        )
        .unwrap()
    }
}

fn digest_with_domain(domain: &[u8], value: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(domain.len() + value.len());
    input.extend_from_slice(domain);
    input.extend_from_slice(value);
    blake3_256(&input)
}

fn feature_digest(features: &[u16]) -> [u8; 32] {
    let mut input = Vec::with_capacity(FEATURE_SET_DOMAIN.len() + features.len() * 2);
    input.extend_from_slice(FEATURE_SET_DOMAIN);
    for feature in features {
        input.extend_from_slice(&feature.to_be_bytes());
    }
    blake3_256(&input)
}

#[test]
fn session_auth_transcript_matches_canonical_fields_and_golden_digest() {
    let fixture = Fixture::new();
    let transcript = fixture.transcript();
    let initiator_credential_digest = signed_object_digest(
        fixture.initiator_credential.transcript_digest(),
        SignatureAlgorithm::Ed25519,
        &fixture.initiator_credential.signature(),
    );
    let responder_credential_digest = signed_object_digest(
        fixture.responder_credential.transcript_digest(),
        SignatureAlgorithm::Ed25519,
        &fixture.responder_credential.signature(),
    );
    let mut expected = CanonicalTranscript::new("crosslab.session-auth.v1").unwrap();
    expected.push(1, 1_u16.to_be_bytes()).unwrap();
    expected.push(2, fixture.owner_id.to_bytes()).unwrap();
    expected
        .push(3, fixture.initiator_credential.device_id().to_bytes())
        .unwrap();
    expected
        .push(4, fixture.initiator_credential.device_key_id().to_bytes())
        .unwrap();
    expected.push(5, initiator_credential_digest).unwrap();
    expected.push(6, [0x21; 32]).unwrap();
    expected
        .push(7, fixture.responder_credential.device_id().to_bytes())
        .unwrap();
    expected
        .push(8, fixture.responder_credential.device_key_id().to_bytes())
        .unwrap();
    expected.push(9, responder_credential_digest).unwrap();
    expected.push(10, [0x22; 32]).unwrap();
    expected.push(11, 1_u16.to_be_bytes()).unwrap();
    expected.push(12, 3_u16.to_be_bytes()).unwrap();
    expected.push(13, feature_digest(&[1, 4, 9])).unwrap();
    expected
        .push(
            14,
            digest_with_domain(CHANNEL_PROFILE_DOMAIN, b"in-process-test"),
        )
        .unwrap();
    expected
        .push(15, digest_with_domain(CHANNEL_VALUE_DOMAIN, &[0x23; 32]))
        .unwrap();

    assert_eq!(transcript.canonical_bytes(), expected.encode());
    assert_eq!(
        transcript.digest(),
        [
            193, 66, 221, 201, 81, 21, 62, 210, 103, 196, 131, 43, 7, 5, 14, 52, 99, 81, 160, 66,
            173, 141, 4, 42, 239, 230, 10, 8, 38, 173, 131, 7,
        ]
    );
}

#[test]
fn proofs_sign_exact_role_label_and_transcript_digest_and_derive_golden_session_id() {
    let fixture = Fixture::new();
    let transcript = fixture.transcript();
    let initiator = transcript
        .create_proof(SessionAuthRole::Initiator, &fixture.initiator_key)
        .unwrap();
    let responder = transcript
        .create_proof(SessionAuthRole::Responder, &fixture.responder_key)
        .unwrap();

    let mut initiator_input = Vec::from(INITIATOR_PROOF_LABEL);
    initiator_input.extend_from_slice(&transcript.digest());
    fixture
        .initiator_key
        .verifying_key()
        .verify_message(&initiator_input, &initiator.signature())
        .unwrap();

    let session_id = transcript
        .derive_session_id(
            &initiator,
            fixture.initiator_key.verifying_key(),
            &responder,
            fixture.responder_key.verifying_key(),
        )
        .unwrap();

    assert_eq!(
        initiator.signature().to_bytes(),
        [
            126, 118, 176, 165, 120, 180, 193, 32, 121, 174, 86, 239, 239, 195, 205, 127, 250, 134,
            211, 156, 86, 246, 34, 203, 97, 0, 126, 62, 238, 120, 92, 140, 200, 213, 161, 176, 102,
            181, 183, 97, 75, 118, 46, 30, 75, 90, 144, 51, 228, 139, 198, 102, 163, 150, 61, 73,
            104, 86, 25, 194, 10, 6, 249, 9,
        ]
    );
    assert_eq!(
        responder.signature().to_bytes(),
        [
            153, 225, 14, 78, 235, 174, 229, 130, 215, 115, 213, 205, 128, 119, 135, 129, 233, 112,
            163, 135, 209, 155, 147, 68, 39, 195, 57, 23, 154, 139, 200, 135, 177, 199, 142, 199, 85,
            229, 25, 233, 62, 210, 158, 9, 206, 183, 63, 241, 193, 148, 219, 34, 188, 75, 65, 134,
            208, 94, 178, 50, 130, 13, 39, 14,
        ]
    );
    assert_eq!(
        session_id.to_bytes(),
        [
            62, 134, 175, 242, 100, 178, 162, 131, 196, 71, 224, 94, 232, 228, 59, 155, 10, 159,
            145, 42, 16, 176, 71, 149, 246, 68, 179, 89, 159, 84, 241, 98,
        ]
    );
}

#[test]
fn wrong_nonce_or_channel_binding_rejects_existing_proof() {
    let fixture = Fixture::new();
    let transcript = fixture.transcript();
    let proof = transcript
        .create_proof(SessionAuthRole::Initiator, &fixture.initiator_key)
        .unwrap();

    let wrong_nonce = SessionAuthTranscriptV1::new(
        fixture.owner_id,
        &fixture.initiator_credential,
        [0xee; 32],
        &fixture.responder_credential,
        [0x22; 32],
        ProtocolVersion::new(1, 3),
        &[1, 4, 9],
        b"in-process-test",
        &[0x23; 32],
    )
    .unwrap();
    assert_eq!(
        wrong_nonce.verify_proof(
            &proof,
            SessionAuthRole::Initiator,
            fixture.initiator_key.verifying_key(),
        ),
        Err(SessionAuthError::WrongProofTranscript)
    );

    let wrong_binding = SessionAuthTranscriptV1::new(
        fixture.owner_id,
        &fixture.initiator_credential,
        [0x21; 32],
        &fixture.responder_credential,
        [0x22; 32],
        ProtocolVersion::new(1, 3),
        &[1, 4, 9],
        b"in-process-test",
        &[0xef; 32],
    )
    .unwrap();
    assert_eq!(
        wrong_binding.verify_proof(
            &proof,
            SessionAuthRole::Initiator,
            fixture.initiator_key.verifying_key(),
        ),
        Err(SessionAuthError::WrongProofTranscript)
    );
}

#[test]
fn wrong_role_or_key_fails_closed() {
    let fixture = Fixture::new();
    let transcript = fixture.transcript();
    let proof = transcript
        .create_proof(SessionAuthRole::Initiator, &fixture.initiator_key)
        .unwrap();

    assert_eq!(
        transcript.verify_proof(
            &proof,
            SessionAuthRole::Responder,
            fixture.initiator_key.verifying_key(),
        ),
        Err(SessionAuthError::WrongProofRole)
    );
    assert_eq!(
        transcript.verify_proof(
            &proof,
            SessionAuthRole::Initiator,
            fixture.responder_key.verifying_key(),
        ),
        Err(SessionAuthError::WrongDeviceKey)
    );
    assert_eq!(
        transcript.create_proof(SessionAuthRole::Initiator, &fixture.responder_key),
        Err(SessionAuthError::WrongDeviceKey)
    );
}

#[test]
fn transcript_rejects_owner_mismatch_and_canonicalizes_feature_order() {
    let fixture = Fixture::new();
    assert_eq!(
        SessionAuthTranscriptV1::new(
            OwnerId::from_bytes([0xff; 32]),
            &fixture.initiator_credential,
            [0x21; 32],
            &fixture.responder_credential,
            [0x22; 32],
            ProtocolVersion::new(1, 3),
            &[1, 4, 9],
            b"in-process-test",
            &[0x23; 32],
        ),
        Err(SessionAuthError::WrongOwner)
    );

    let canonical = fixture.transcript();
    let reordered = SessionAuthTranscriptV1::new(
        fixture.owner_id,
        &fixture.initiator_credential,
        [0x21; 32],
        &fixture.responder_credential,
        [0x22; 32],
        ProtocolVersion::new(1, 3),
        &[4, 9, 1],
        b"in-process-test",
        &[0x23; 32],
    )
    .unwrap();
    assert_eq!(canonical.digest(), reordered.digest());
}
