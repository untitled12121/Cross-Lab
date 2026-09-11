use crosslab_policy::{CapabilityId, OperationName};

#[test]
fn capability_id_accepts_canonical_dotted_ascii() {
    let id = CapabilityId::parse("files.transfer").unwrap();
    assert_eq!(id.as_str(), "files.transfer");
}

#[test]
fn capability_id_rejects_noncanonical_values_and_length_overflow() {
    for invalid in [
        "clipboard",
        ".clipboard.read",
        "clipboard.",
        "clipboard..read",
        "Clipboard.read",
        "clipboard.Read",
        "clipboard._read",
        "clipboard.-read",
        "clipboard.read-",
        "clipboard.réad",
        "clipboard read",
    ] {
        assert!(CapabilityId::parse(invalid).is_err(), "accepted {invalid:?}");
    }

    let oversized = format!("a.{}", "b".repeat(127));
    assert!(oversized.len() > 128);
    assert!(CapabilityId::parse(&oversized).is_err());
}

#[test]
fn operation_name_accepts_one_canonical_segment() {
    let operation = OperationName::parse("receive-file").unwrap();
    assert_eq!(operation.as_str(), "receive-file");
}

#[test]
fn operation_name_rejects_invalid_segments_and_length_overflow() {
    for invalid in [
        "",
        "receive.file",
        "Receive",
        "receive_file",
        "-receive",
        "receive-",
        "réceive",
        "receive file",
    ] {
        assert!(OperationName::parse(invalid).is_err(), "accepted {invalid:?}");
    }

    let oversized = "a".repeat(65);
    assert!(OperationName::parse(&oversized).is_err());
}
