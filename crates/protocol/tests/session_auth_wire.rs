use crosslab_crypto::{SignatureAlgorithm, SigningKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_protocol::wire::v1::{
    SessionAuthBootstrapV1, SessionAuthRoleV1, SignatureAlgorithmV1, session_auth_bootstrap_v1,
};
use crosslab_protocol::{
    FeatureSet, FrameLimit, ProtocolRange, ProtocolWireError, SESSION_AUTH_PROFILE_V1,
    SessionAuthBootstrapMessage, SessionAuthHello, SessionAuthProofMessage, SessionAuthRole,
    decode_session_auth_bootstrap, encode_frame, encode_session_auth_bootstrap,
};
use prost::Message;

fn credential() -> DeviceCredential {
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
    let device_key = SigningKey::from_secret_bytes([0x13; 32]);
    DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([0x14; 32]),
        &device_key,
        3,
        &root,
        &delegation,
        &issuer_key,
    )
    .unwrap()
}

fn hello() -> SessionAuthHello {
    SessionAuthHello::new(
        credential(),
        vec![ProtocolRange::new(1, 0, 4).unwrap()],
        FeatureSet::new(&[1, 3, 7], &[1]).unwrap(),
        [0x21; 32],
    )
}

fn proof() -> SessionAuthProofMessage {
    let key = SigningKey::from_secret_bytes([0x22; 32]);
    SessionAuthProofMessage::new(
        SessionAuthRole::Initiator,
        [0x23; 32],
        key.sign_digest(&[0x24; 32]),
    )
}

fn frame(wire: &SessionAuthBootstrapV1) -> Vec<u8> {
    encode_frame(&wire.encode_to_vec(), FrameLimit::BootstrapHello).unwrap()
}

fn raw_hello() -> SessionAuthBootstrapV1 {
    SessionAuthBootstrapV1::from(&SessionAuthBootstrapMessage::Hello(hello()))
}

fn raw_proof() -> SessionAuthBootstrapV1 {
    SessionAuthBootstrapV1::from(&SessionAuthBootstrapMessage::Proof(proof()))
}

#[test]
fn session_auth_hello_and_proof_round_trip_outside_control_envelope() {
    let messages = [
        SessionAuthBootstrapMessage::Hello(hello()),
        SessionAuthBootstrapMessage::Proof(proof()),
    ];

    for message in messages {
        let encoded = encode_session_auth_bootstrap(&message).unwrap();
        assert_eq!(decode_session_auth_bootstrap(&encoded).unwrap(), message);
    }
}

#[test]
fn unknown_profile_or_missing_body_is_rejected() {
    let mut unknown = raw_hello();
    unknown.session_auth_profile = u32::from(SESSION_AUTH_PROFILE_V1) + 1;
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&unknown)),
        Err(ProtocolWireError::InvalidSessionAuthProfile(2))
    );

    let missing = SessionAuthBootstrapV1 {
        session_auth_profile: u32::from(SESSION_AUTH_PROFILE_V1),
        body: None,
    };
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&missing)),
        Err(ProtocolWireError::MissingSessionAuthBootstrapBody)
    );
}

#[test]
fn hello_rejects_identifier_nonce_and_credential_mismatch() {
    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello.owner_id.pop();
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidOwnerIdLength(31))
    );

    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello.device_id.pop();
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidDeviceIdLength(31))
    );

    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello.nonce.pop();
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidSessionAuthNonceLength(31))
    );

    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello.owner_id = vec![0xee; 32];
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::CredentialIdentityMismatch)
    );
}

#[test]
fn hello_rejects_malformed_device_credential() {
    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello
        .device_credential
        .as_mut()
        .unwrap()
        .device_key_id
        .pop();
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidKeyIdLength(31))
    );

    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello
        .device_credential
        .as_mut()
        .unwrap()
        .signature_algorithm = SignatureAlgorithmV1::Unspecified as i32;
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidSignatureAlgorithm(0))
    );

    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello.device_credential.as_mut().unwrap().signature.pop();
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidSignatureLength(63))
    );
}

#[test]
fn hello_enforces_protocol_and_feature_collection_bounds() {
    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    let range = hello.protocol_ranges[0].clone();
    hello.protocol_ranges = vec![range; 9];
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::TooManyProtocolRanges(9))
    );

    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello.protocol_ranges[0].min_minor = 5;
    hello.protocol_ranges[0].max_minor = 4;
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidProtocolRange)
    );

    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello.supported_features = (0..65).collect();
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::TooManySupportedFeatures(65))
    );

    let mut raw = raw_hello();
    let session_auth_bootstrap_v1::Body::Hello(hello) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    hello.required_features = (0..33).collect();
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::TooManyRequiredFeatures(33))
    );
}

#[test]
fn proof_rejects_role_digest_algorithm_and_signature_errors() {
    let mut raw = raw_proof();
    let session_auth_bootstrap_v1::Body::Proof(proof) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    proof.role = SessionAuthRoleV1::Unspecified as i32;
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidSessionAuthRole(0))
    );

    let mut raw = raw_proof();
    let session_auth_bootstrap_v1::Body::Proof(proof) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    proof.transcript_digest.pop();
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidSessionAuthTranscriptDigestLength(
            31
        ))
    );

    let mut raw = raw_proof();
    let session_auth_bootstrap_v1::Body::Proof(proof) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    proof.signature_algorithm = SignatureAlgorithmV1::Unspecified as i32;
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidSignatureAlgorithm(0))
    );

    let mut raw = raw_proof();
    let session_auth_bootstrap_v1::Body::Proof(proof) = raw.body.as_mut().unwrap() else {
        unreachable!();
    };
    proof.signature.pop();
    assert_eq!(
        decode_session_auth_bootstrap(&frame(&raw)),
        Err(ProtocolWireError::InvalidSignatureLength(63))
    );
}

#[test]
fn session_auth_bootstrap_frame_limit_is_checked_before_protobuf_decode() {
    let mut frame = Vec::with_capacity(4);
    frame.extend_from_slice(&65_537_u32.to_be_bytes());

    assert!(matches!(
        decode_session_auth_bootstrap(&frame),
        Err(ProtocolWireError::Frame(_))
    ));
}

#[test]
fn imported_credential_keeps_the_v1_algorithm_contract() {
    let hello = hello();
    assert_eq!(hello.device_credential().owner_id(), hello.owner_id());
    assert_eq!(hello.device_credential().device_id(), hello.device_id());
    assert_eq!(SignatureAlgorithm::Ed25519.code(), 1);
}
