use crosslab_core::{StreamOpenError, StreamSendError};

#[test]
fn stream_open_error_preserves_frame_without_debug_leak() {
    let frame = b"crosslab-sensitive-opening-frame".to_vec();
    let error = StreamOpenError::Full(frame.clone());
    let debug = format!("{error:?}");
    let display = error.to_string();

    assert!(!debug.contains("crosslab-sensitive-opening-frame"));
    assert!(!display.contains("crosslab-sensitive-opening-frame"));
    assert_eq!(error.into_opening_frame(), frame);
}

#[test]
fn stream_send_error_preserves_chunk_without_debug_leak() {
    let chunk = b"crosslab-sensitive-stream-chunk".to_vec();
    let error = StreamSendError::Full(chunk.clone());
    let debug = format!("{error:?}");
    let display = error.to_string();

    assert!(!debug.contains("crosslab-sensitive-stream-chunk"));
    assert!(!display.contains("crosslab-sensitive-stream-chunk"));
    assert_eq!(error.into_chunk(), chunk);
}
