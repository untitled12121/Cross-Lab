use crosslab_crypto::{
    CanonicalError, CanonicalTranscript, SignatureAlgorithm, SigningKey, blake3_256,
    signed_object_digest,
};

#[test]
fn canonical_transcript_has_exact_v1_layout() {
    let mut transcript = CanonicalTranscript::new("crosslab.test.v1").unwrap();
    transcript.push(1, [0xAA]).unwrap();
    transcript.push(2, [0xBB, 0xCC]).unwrap();

    let mut expected = Vec::new();
    expected.extend_from_slice(b"crosslab-canon-1\0");
    expected.extend_from_slice(&(16_u16).to_be_bytes());
    expected.extend_from_slice(b"crosslab.test.v1");
    expected.extend_from_slice(&(2_u16).to_be_bytes());
    expected.extend_from_slice(&(1_u16).to_be_bytes());
    expected.extend_from_slice(&(1_u32).to_be_bytes());
    expected.push(0xAA);
    expected.extend_from_slice(&(2_u16).to_be_bytes());
    expected.extend_from_slice(&(2_u32).to_be_bytes());
    expected.extend_from_slice(&[0xBB, 0xCC]);

    assert_eq!(transcript.encode(), expected);
}

#[test]
fn canonical_transcript_rejects_zero_and_non_increasing_tags() {
    let mut transcript = CanonicalTranscript::new("crosslab.test.v1").unwrap();
    assert_eq!(transcript.push(0, [1]), Err(CanonicalError::InvalidTag));

    transcript.push(2, [1]).unwrap();
    assert_eq!(
        transcript.push(2, [2]),
        Err(CanonicalError::NonIncreasingTag)
    );
    assert_eq!(
        transcript.push(1, [3]),
        Err(CanonicalError::NonIncreasingTag)
    );
}

#[test]
fn transcript_digest_is_domain_separated() {
    let mut first = CanonicalTranscript::new("crosslab.first.v1").unwrap();
    first.push(1, [7]).unwrap();

    let mut second = CanonicalTranscript::new("crosslab.second.v1").unwrap();
    second.push(1, [7]).unwrap();

    assert_ne!(first.digest(), second.digest());
}

#[test]
fn ed25519_signs_the_digest_and_rejects_modified_input() {
    let signing_key = SigningKey::from_secret_bytes([7_u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let digest = blake3_256(b"crosslab test digest");
    let signature = signing_key.sign_digest(&digest);

    verifying_key.verify_digest(&digest, &signature).unwrap();

    let mut modified = digest;
    modified[0] ^= 1;
    assert!(verifying_key.verify_digest(&modified, &signature).is_err());
}

#[test]
fn signing_key_debug_never_exposes_secret_material() {
    let signing_key = SigningKey::from_secret_bytes([0xA5; 32]);
    let debug = format!("{signing_key:?}");

    assert!(debug.contains("REDACTED"));
    assert!(!debug.contains("a5a5"));
}

#[test]
fn signed_object_digest_binds_algorithm_and_signature() {
    let signing_key = SigningKey::from_secret_bytes([11_u8; 32]);
    let digest = blake3_256(b"object");
    let signature = signing_key.sign_digest(&digest);

    let object_digest = signed_object_digest(digest, SignatureAlgorithm::Ed25519, &signature);
    let different_digest = signed_object_digest(
        blake3_256(b"other object"),
        SignatureAlgorithm::Ed25519,
        &signature,
    );

    assert_ne!(object_digest, different_digest);
}
