use crosslab_crypto::{Signature, SigningKey};
use crosslab_identity::{DeviceId, KeyId, OwnerId};
use crosslab_protocol::{
    FrameError, PairingBootstrapMessage, PairingConfirmation, PairingCredentialAccepted,
    PairingHello, PairingRole, ProtocolWireError, decode_pairing_bootstrap,
    encode_pairing_bootstrap, encode_frame, wire,
};
use prost::Message;

fn public_key(secret: u8) -> [u8; 32] {
    SigningKey::from_secret_bytes([secret; 32])
        .verifying_key()
        .to_bytes()
}

fn hello() -> PairingHello {
    PairingHello::new(
        PairingRole::Joiner,
        1,
        [0x11; 16],
        OwnerId::from_bytes([0x22; 32]),
        DeviceId::from_bytes([0x33; 32]),
        SigningKey::from_secret_bytes([0x44; 32]).verifying_key(),
        [0x55; 32],
    )
}

fn confirmation() -> PairingConfirmation {
    PairingConfirmation::new(PairingRole::Joiner, [0x11; 16], [0x66; 32])
}

fn credential_accepted() -> PairingCredentialAccepted {
    PairingCredentialAccepted::new(
        [0x11; 16],
        [0x77; 32],
        [0x88; 32],
        DeviceId::from_bytes([0x33; 32]),
        KeyId::from_bytes([0x99; 32]),
        Signature::from_bytes([0xaa; 64]),
    )
}

fn raw_hello() -> wire::v1::PairingHelloV1 {
    wire::v1::PairingHelloV1 {
        role: wire::v1::PairingRoleV1::Joiner as i32,
        protocol_major: 1,
        pairing_id: vec![0x11; 16],
        owner_id: vec![0x22; 32],
        device_id: vec![0x33; 32],
        device_algorithm: wire::v1::SignatureAlgorithmV1::Ed25519 as i32,
        device_public_key: public_key(0x44).to_vec(),
        nonce: vec![0x55; 32],
    }
}

fn raw_confirmation() -> wire::v1::PairingConfirmationV1 {
    wire::v1::PairingConfirmationV1 {
        role: wire::v1::PairingRoleV1::Joiner as i32,
        pairing_id: vec![0x11; 16],
        confirmation: vec![0x66; 32],
    }
}

fn raw_credential_accepted() -> wire::v1::PairingCredentialAcceptedV1 {
    wire::v1::PairingCredentialAcceptedV1 {
        pairing_id: vec![0x11; 16],
        pairing_transcript_digest: vec![0x77; 32],
        device_credential_signed_object_digest: vec![0x88; 32],
        joiner_device_id: vec![0x33; 32],
        joiner_device_key_id: vec![0x99; 32],
        signature_algorithm: wire::v1::SignatureAlgorithmV1::Ed25519 as i32,
        signature: vec![0xaa; 64],
    }
}

fn raw_bootstrap(body: wire::v1::pairing_bootstrap_v1::Body) -> wire::v1::PairingBootstrapV1 {
    wire::v1::PairingBootstrapV1 {
        pairing_profile: 1,
        body: Some(body),
    }
}

fn frame(message: wire::v1::PairingBootstrapV1) -> Vec<u8> {
    encode_frame(
        &message.encode_to_vec(),
        crosslab_protocol::FrameLimit::BootstrapHello,
    )
    .unwrap()
}

#[test]
fn pairing_bootstrap_messages_round_trip_outside_the_session_envelope() {
    let messages = [
        PairingBootstrapMessage::Hello(hello()),
        PairingBootstrapMessage::Confirmation(confirmation()),
        PairingBootstrapMessage::CredentialAccepted(credential_accepted()),
    ];

    for expected in messages {
        let encoded = encode_pairing_bootstrap(&expected).unwrap();
        assert_eq!(decode_pairing_bootstrap(&encoded).unwrap(), expected);
    }
}

#[test]
fn pairing_bootstrap_rejects_unknown_profile_and_missing_body() {
    let mut unknown_profile = raw_bootstrap(wire::v1::pairing_bootstrap_v1::Body::Hello(
        raw_hello(),
    ));
    unknown_profile.pairing_profile = 2;
    assert_eq!(
        decode_pairing_bootstrap(&frame(unknown_profile)).unwrap_err(),
        ProtocolWireError::InvalidPairingProfile(2)
    );

    let missing_body = wire::v1::PairingBootstrapV1 {
        pairing_profile: 1,
        body: None,
    };
    assert_eq!(
        decode_pairing_bootstrap(&frame(missing_body)).unwrap_err(),
        ProtocolWireError::MissingPairingBootstrapBody
    );
}

#[test]
fn pairing_hello_rejects_malformed_security_fields() {
    let cases = [
        (
            {
                let mut value = raw_hello();
                value.pairing_id.pop();
                value
            },
            ProtocolWireError::InvalidPairingIdLength(15),
        ),
        (
            {
                let mut value = raw_hello();
                value.owner_id.pop();
                value
            },
            ProtocolWireError::InvalidOwnerIdLength(31),
        ),
        (
            {
                let mut value = raw_hello();
                value.device_id.pop();
                value
            },
            ProtocolWireError::InvalidDeviceIdLength(31),
        ),
        (
            {
                let mut value = raw_hello();
                value.device_public_key.pop();
                value
            },
            ProtocolWireError::InvalidDevicePublicKeyLength(31),
        ),
        (
            {
                let mut value = raw_hello();
                value.nonce.pop();
                value
            },
            ProtocolWireError::InvalidPairingNonceLength(31),
        ),
    ];

    for (hello, expected_error) in cases {
        let bootstrap = raw_bootstrap(wire::v1::pairing_bootstrap_v1::Body::Hello(hello));
        assert_eq!(
            decode_pairing_bootstrap(&frame(bootstrap)).unwrap_err(),
            expected_error
        );
    }

    let mut invalid_role = raw_hello();
    invalid_role.role = 99;
    let bootstrap = raw_bootstrap(wire::v1::pairing_bootstrap_v1::Body::Hello(invalid_role));
    assert_eq!(
        decode_pairing_bootstrap(&frame(bootstrap)).unwrap_err(),
        ProtocolWireError::InvalidPairingRole(99)
    );

    let mut invalid_algorithm = raw_hello();
    invalid_algorithm.device_algorithm = 99;
    let bootstrap = raw_bootstrap(wire::v1::pairing_bootstrap_v1::Body::Hello(
        invalid_algorithm,
    ));
    assert_eq!(
        decode_pairing_bootstrap(&frame(bootstrap)).unwrap_err(),
        ProtocolWireError::InvalidSignatureAlgorithm(99)
    );

    let mut invalid_key = raw_hello();
    invalid_key.device_public_key = [0_u8; 32].to_vec();
    let bootstrap = raw_bootstrap(wire::v1::pairing_bootstrap_v1::Body::Hello(invalid_key));
    assert_eq!(
        decode_pairing_bootstrap(&frame(bootstrap)).unwrap_err(),
        ProtocolWireError::InvalidDevicePublicKey
    );
}

#[test]
fn pairing_confirmation_rejects_wrong_role_and_lengths() {
    let mut invalid_role = raw_confirmation();
    invalid_role.role = 99;
    let bootstrap = raw_bootstrap(wire::v1::pairing_bootstrap_v1::Body::Confirmation(
        invalid_role,
    ));
    assert_eq!(
        decode_pairing_bootstrap(&frame(bootstrap)).unwrap_err(),
        ProtocolWireError::InvalidPairingRole(99)
    );

    let mut invalid_pairing_id = raw_confirmation();
    invalid_pairing_id.pairing_id.pop();
    let bootstrap = raw_bootstrap(wire::v1::pairing_bootstrap_v1::Body::Confirmation(
        invalid_pairing_id,
    ));
    assert_eq!(
        decode_pairing_bootstrap(&frame(bootstrap)).unwrap_err(),
        ProtocolWireError::InvalidPairingIdLength(15)
    );

    let mut invalid_confirmation = raw_confirmation();
    invalid_confirmation.confirmation.pop();
    let bootstrap = raw_bootstrap(wire::v1::pairing_bootstrap_v1::Body::Confirmation(
        invalid_confirmation,
    ));
    assert_eq!(
        decode_pairing_bootstrap(&frame(bootstrap)).unwrap_err(),
        ProtocolWireError::InvalidPairingConfirmationLength(31)
    );
}

#[test]
fn pairing_credential_accepted_rejects_malformed_proof_fields() {
    let cases = [
        (
            {
                let mut value = raw_credential_accepted();
                value.pairing_id.pop();
                value
            },
            ProtocolWireError::InvalidPairingIdLength(15),
        ),
        (
            {
                let mut value = raw_credential_accepted();
                value.pairing_transcript_digest.pop();
                value
            },
            ProtocolWireError::InvalidPairingTranscriptDigestLength(31),
        ),
        (
            {
                let mut value = raw_credential_accepted();
                value.device_credential_signed_object_digest.pop();
                value
            },
            ProtocolWireError::InvalidSignedObjectDigestLength(31),
        ),
        (
            {
                let mut value = raw_credential_accepted();
                value.joiner_device_id.pop();
                value
            },
            ProtocolWireError::InvalidDeviceIdLength(31),
        ),
        (
            {
                let mut value = raw_credential_accepted();
                value.joiner_device_key_id.pop();
                value
            },
            ProtocolWireError::InvalidKeyIdLength(31),
        ),
        (
            {
                let mut value = raw_credential_accepted();
                value.signature.pop();
                value
            },
            ProtocolWireError::InvalidSignatureLength(63),
        ),
    ];

    for (accepted, expected_error) in cases {
        let bootstrap = raw_bootstrap(
            wire::v1::pairing_bootstrap_v1::Body::CredentialAccepted(accepted),
        );
        assert_eq!(
            decode_pairing_bootstrap(&frame(bootstrap)).unwrap_err(),
            expected_error
        );
    }

    let mut invalid_algorithm = raw_credential_accepted();
    invalid_algorithm.signature_algorithm = 99;
    let bootstrap = raw_bootstrap(
        wire::v1::pairing_bootstrap_v1::Body::CredentialAccepted(invalid_algorithm),
    );
    assert_eq!(
        decode_pairing_bootstrap(&frame(bootstrap)).unwrap_err(),
        ProtocolWireError::InvalidSignatureAlgorithm(99)
    );
}

#[test]
fn pairing_bootstrap_frame_limit_is_checked_before_protobuf_decode() {
    let declared = crosslab_protocol::FrameLimit::BootstrapHello.max_payload_len() + 1;
    let frame = (declared as u32).to_be_bytes();

    assert_eq!(
        decode_pairing_bootstrap(&frame).unwrap_err(),
        ProtocolWireError::Frame(FrameError::FrameTooLarge {
            declared,
            max: crosslab_protocol::FrameLimit::BootstrapHello.max_payload_len(),
        })
    );
}
