use crosslab_policy::OperationId;
use crosslab_protocol::{
    FILE_TRANSFER_CHECKPOINT_BYTES, FileTransferAcceptance, FileTransferDigest, FileTransferOffer,
    FileTransferProfileError, FileTransferResult, FileTransferTerminalOutcome,
    FileTransferWireError, MAX_FILE_TRANSFER_OFFER_WIRE_BYTES, TransferId,
    decode_file_transfer_acceptance, decode_file_transfer_offer, decode_file_transfer_result,
    encode_file_transfer_acceptance, encode_file_transfer_offer, encode_file_transfer_result,
    valid_resume_offset, wire,
};
use prost::Message;

fn transfer_id() -> TransferId {
    TransferId::from_bytes([0x11; 32])
}

fn digest() -> FileTransferDigest {
    FileTransferDigest::from_bytes([0x22; 32])
}

#[test]
fn offer_has_stable_v2_protobuf_vector() {
    let offer = FileTransferOffer::new(transfer_id(), "a.txt".into(), 3, digest()).unwrap();
    let mut expected = vec![0x08, 0x02, 0x12, 0x20];
    expected.extend_from_slice(&[0x11; 32]);
    expected.extend_from_slice(&[0x1a, 0x05]);
    expected.extend_from_slice(b"a.txt");
    expected.extend_from_slice(&[0x20, 0x03, 0x2a, 0x20]);
    expected.extend_from_slice(&[0x22; 32]);

    let encoded = encode_file_transfer_offer(&offer).unwrap();

    assert_eq!(encoded, expected);
    assert_eq!(decode_file_transfer_offer(&encoded).unwrap(), offer);
}

#[test]
fn offer_rejects_unsafe_or_oversized_display_names() {
    for name in ["", ".", "..", "a/b", "a\\b", "a\0b"] {
        assert_eq!(
            FileTransferOffer::new(transfer_id(), name.into(), 0, digest()),
            Err(FileTransferProfileError::InvalidDisplayName)
        );
    }
    assert_eq!(
        FileTransferOffer::new(transfer_id(), "a".repeat(256), 0, digest()),
        Err(FileTransferProfileError::InvalidDisplayName)
    );
}

#[test]
fn offer_debug_redacts_filename_and_digest() {
    let offer =
        FileTransferOffer::new(transfer_id(), "private-document.txt".into(), 42, digest()).unwrap();

    let rendered = format!("{offer:?}");

    assert!(!rendered.contains("private-document.txt"));
    assert!(!rendered.contains("[34, 34"));
    assert!(rendered.contains("display_name_len"));
    assert!(rendered.contains("[REDACTED; 32 bytes]"));
}

#[test]
fn resume_offsets_follow_durable_checkpoint_rule() {
    let file_size = FILE_TRANSFER_CHECKPOINT_BYTES + 13;

    assert!(valid_resume_offset(0, file_size));
    assert!(valid_resume_offset(
        FILE_TRANSFER_CHECKPOINT_BYTES,
        file_size
    ));
    assert!(valid_resume_offset(file_size, file_size));
    assert!(!valid_resume_offset(1, file_size));
    assert!(!valid_resume_offset(file_size + 1, file_size));
}

#[test]
fn ready_and_already_complete_round_trip() {
    let ready = FileTransferAcceptance::Ready {
        transfer_id: transfer_id(),
        resume_offset: FILE_TRANSFER_CHECKPOINT_BYTES,
        operation_id: OperationId::from_bytes([0x33; 32]),
    };
    let complete = FileTransferAcceptance::AlreadyComplete {
        transfer_id: transfer_id(),
    };

    let encoded_ready = encode_file_transfer_acceptance(&ready).unwrap();
    let encoded_complete = encode_file_transfer_acceptance(&complete).unwrap();

    assert_eq!(
        decode_file_transfer_acceptance(&encoded_ready).unwrap(),
        ready
    );
    assert_eq!(
        decode_file_transfer_acceptance(&encoded_complete).unwrap(),
        complete
    );
}

#[test]
fn terminal_results_round_trip_and_reject_unknown_outcomes() {
    for outcome in [
        FileTransferTerminalOutcome::Completed,
        FileTransferTerminalOutcome::Cancelled,
        FileTransferTerminalOutcome::IntegrityFailed,
        FileTransferTerminalOutcome::StorageFailed,
    ] {
        let result = FileTransferResult::new(transfer_id(), outcome);
        let encoded = encode_file_transfer_result(result).unwrap();
        assert_eq!(decode_file_transfer_result(&encoded).unwrap(), result);
    }

    let unknown = wire::v1::FileTransferResultV2 {
        profile_version: 2,
        transfer_id: transfer_id().to_bytes().to_vec(),
        outcome: 99,
    }
    .encode_to_vec();
    assert_eq!(
        decode_file_transfer_result(&unknown),
        Err(FileTransferWireError::InvalidTerminalOutcome(99))
    );
}

#[test]
fn decode_rejects_payload_before_unbounded_protobuf_allocation() {
    let payload = vec![0; MAX_FILE_TRANSFER_OFFER_WIRE_BYTES + 1];

    assert_eq!(
        decode_file_transfer_offer(&payload),
        Err(FileTransferWireError::PayloadTooLarge {
            actual: MAX_FILE_TRANSFER_OFFER_WIRE_BYTES + 1,
            max: MAX_FILE_TRANSFER_OFFER_WIRE_BYTES,
        })
    );
}

#[test]
fn decode_rejects_wrong_profile_and_identifier_lengths() {
    let wrong_profile = wire::v1::FileTransferOfferV2 {
        profile_version: 3,
        transfer_id: transfer_id().to_bytes().to_vec(),
        display_name: "a.txt".into(),
        file_size: 0,
        blake3_digest: digest().to_bytes().to_vec(),
    }
    .encode_to_vec();
    assert_eq!(
        decode_file_transfer_offer(&wrong_profile),
        Err(FileTransferWireError::InvalidProfile(3))
    );

    let short_id = wire::v1::FileTransferOfferV2 {
        profile_version: 2,
        transfer_id: vec![0x11; 31],
        display_name: "a.txt".into(),
        file_size: 0,
        blake3_digest: digest().to_bytes().to_vec(),
    }
    .encode_to_vec();
    assert_eq!(
        decode_file_transfer_offer(&short_id),
        Err(FileTransferWireError::InvalidTransferIdLength(31))
    );
}
