use crosslab_protocol::{FrameError, FrameLimit, decode_frame, encode_frame};

#[test]
fn frame_limits_match_protocol_v1() {
    assert_eq!(FrameLimit::BootstrapHello.max_payload_len(), 65_536);
    assert_eq!(FrameLimit::NormalControl.max_payload_len(), 262_144);
    assert_eq!(FrameLimit::DataStreamOpen.max_payload_len(), 4_096);
}

#[test]
fn frame_uses_u32_big_endian_length_prefix() {
    let encoded = encode_frame(b"abc", FrameLimit::NormalControl).unwrap();

    assert_eq!(encoded, vec![0, 0, 0, 3, b'a', b'b', b'c']);
    assert_eq!(
        decode_frame(&encoded, FrameLimit::NormalControl).unwrap(),
        b"abc"
    );
}

#[test]
fn oversized_declared_length_is_rejected_before_payload_is_present() {
    let frame = 4_097_u32.to_be_bytes();

    assert_eq!(
        decode_frame(&frame, FrameLimit::DataStreamOpen).unwrap_err(),
        FrameError::FrameTooLarge {
            declared: 4_097,
            max: 4_096,
        }
    );
}

#[test]
fn oversized_payload_is_rejected_on_encode() {
    let payload = vec![0_u8; 4_097];

    assert_eq!(
        encode_frame(&payload, FrameLimit::DataStreamOpen).unwrap_err(),
        FrameError::FrameTooLarge {
            declared: 4_097,
            max: 4_096,
        }
    );
}

#[test]
fn truncated_and_trailing_frames_are_rejected() {
    let truncated = [0, 0, 0, 3, 1, 2];
    let trailing = [0, 0, 0, 1, 1, 2];

    assert_eq!(
        decode_frame(&truncated, FrameLimit::NormalControl).unwrap_err(),
        FrameError::Truncated {
            declared: 3,
            available: 2,
        }
    );
    assert_eq!(
        decode_frame(&trailing, FrameLimit::NormalControl).unwrap_err(),
        FrameError::TrailingBytes {
            declared: 1,
            available: 2,
        }
    );
}

#[test]
fn missing_length_prefix_is_rejected() {
    assert_eq!(
        decode_frame(&[0, 0, 0], FrameLimit::NormalControl).unwrap_err(),
        FrameError::MissingLengthPrefix
    );
}
