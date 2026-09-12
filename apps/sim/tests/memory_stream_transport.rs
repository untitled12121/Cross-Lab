use std::num::NonZeroUsize;

use crosslab_core::{
    StreamAcceptError, StreamOpenError, StreamReceiveError, StreamSendError, TransportConnection,
};
use crosslab_sim::transport::{MemoryTransportConfig, MemoryTransportPair};

fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

fn pair(
    stream_capacity: usize,
    chunk_capacity: usize,
    max_chunk_bytes: usize,
) -> MemoryTransportPair {
    MemoryTransportPair::with_config(
        MemoryTransportConfig::new(
            nonzero(4),
            nonzero(stream_capacity),
            nonzero(chunk_capacity),
            nonzero(max_chunk_bytes),
        ),
        [0x51; 32],
    )
}

#[test]
fn uni_stream_open_and_chunks_are_ordered_and_bounded() {
    let pair = pair(2, 2, 4);
    let (a, b) = pair.endpoints();
    let mut send = a.try_open_uni_stream(vec![1, 2, 3]).unwrap();
    let incoming = b.try_accept_uni_stream().unwrap();
    assert_eq!(incoming.opening_frame(), &[1, 2, 3]);
    let (_, mut receive) = incoming.into_parts();

    send.try_send_chunk(vec![10]).unwrap();
    send.try_send_chunk(vec![11]).unwrap();
    assert_eq!(
        send.try_send_chunk(vec![12]).unwrap_err().into_chunk(),
        vec![12]
    );
    assert_eq!(receive.try_receive_chunk().unwrap(), vec![10]);
    assert_eq!(receive.try_receive_chunk().unwrap(), vec![11]);
    assert_eq!(
        receive.try_receive_chunk().unwrap_err(),
        StreamReceiveError::Empty
    );
}

#[test]
fn pending_open_saturation_preserves_frame_and_failed_open_leaves_no_stream() {
    let pair = pair(1, 1, 8);
    let (a, b) = pair.endpoints();
    let _first = a.try_open_uni_stream(vec![1]).unwrap();
    let second_frame = vec![2];
    let error = match a.try_open_uni_stream(second_frame.clone()) {
        Err(error) => error,
        Ok(_) => panic!("second open unexpectedly bypassed stream capacity"),
    };
    assert_eq!(error.into_opening_frame(), second_frame);

    let accepted = b.try_accept_uni_stream().unwrap();
    let (_, mut first_receive) = accepted.into_parts();
    first_receive.cancel();

    let _second = a.try_open_uni_stream(vec![3]).unwrap();
    assert_eq!(b.try_accept_uni_stream().unwrap().opening_frame(), &[3]);
}

#[test]
fn oversized_opening_frame_is_rejected_without_allocating_stream_state() {
    let config = MemoryTransportConfig::new(nonzero(4), nonzero(1), nonzero(1), nonzero(8))
        .with_frame_limits(nonzero(8), nonzero(4));
    let pair = MemoryTransportPair::with_config(config, [0x52; 32]);
    let (a, b) = pair.endpoints();

    let opening = vec![0x31; 5];
    let error = match a.try_open_uni_stream(opening.clone()) {
        Err(error) => error,
        Ok(_) => panic!("oversized opening frame unexpectedly allocated a stream"),
    };
    assert_eq!(error, StreamOpenError::TooLarge(opening));
    assert_eq!(b.try_accept_uni_stream(), Err(StreamAcceptError::Empty));

    let _stream = a.try_open_uni_stream(vec![0x32; 4]).unwrap();
    assert_eq!(
        b.try_accept_uni_stream().unwrap().opening_frame(),
        &[0x32; 4]
    );
}

#[test]
fn oversized_and_full_chunks_preserve_unsent_bytes() {
    let pair = pair(2, 1, 2);
    let (a, b) = pair.endpoints();
    let mut send = a.try_open_uni_stream(vec![1]).unwrap();
    let (_, mut receive) = b.try_accept_uni_stream().unwrap().into_parts();

    let oversized = vec![1, 2, 3];
    assert_eq!(
        send.try_send_chunk(oversized.clone())
            .unwrap_err()
            .into_chunk(),
        oversized
    );

    send.try_send_chunk(vec![4]).unwrap();
    let full = vec![5];
    assert_eq!(
        send.try_send_chunk(full.clone()).unwrap_err().into_chunk(),
        full
    );
    assert_eq!(receive.try_receive_chunk().unwrap(), vec![4]);
}

#[test]
fn graceful_finish_drains_before_finished() {
    let pair = pair(2, 2, 8);
    let (a, b) = pair.endpoints();
    let mut send = a.try_open_uni_stream(vec![1]).unwrap();
    let (_, mut receive) = b.try_accept_uni_stream().unwrap().into_parts();

    send.try_send_chunk(vec![7]).unwrap();
    send.finish();
    assert_eq!(receive.try_receive_chunk().unwrap(), vec![7]);
    assert_eq!(
        receive.try_receive_chunk().unwrap_err(),
        StreamReceiveError::Finished
    );
    assert_eq!(
        send.try_send_chunk(vec![8]).unwrap_err().into_chunk(),
        vec![8]
    );
}

#[test]
fn cancellation_clears_queued_chunks_and_propagates() {
    let pair = pair(2, 2, 8);
    let (a, b) = pair.endpoints();
    let mut send = a.try_open_uni_stream(vec![1]).unwrap();
    let (_, mut receive) = b.try_accept_uni_stream().unwrap().into_parts();

    send.try_send_chunk(vec![9]).unwrap();
    send.cancel();
    assert_eq!(
        receive.try_receive_chunk().unwrap_err(),
        StreamReceiveError::Cancelled
    );
    assert_eq!(
        send.try_send_chunk(vec![10]).unwrap_err().into_chunk(),
        vec![10]
    );
}

#[test]
fn receive_cancel_propagates_to_sender() {
    let pair = pair(2, 2, 8);
    let (a, b) = pair.endpoints();
    let mut send = a.try_open_uni_stream(vec![1]).unwrap();
    let (_, mut receive) = b.try_accept_uni_stream().unwrap().into_parts();

    receive.cancel();
    assert_eq!(
        send.try_send_chunk(vec![10]).unwrap_err().into_chunk(),
        vec![10]
    );
    assert_eq!(
        receive.try_receive_chunk().unwrap_err(),
        StreamReceiveError::Cancelled
    );
}

#[test]
fn connection_close_cancels_pending_and_active_streams() {
    let pair = pair(2, 2, 8);
    let (a, b) = pair.endpoints();
    let mut active_send = a.try_open_uni_stream(vec![1]).unwrap();
    let (_, mut active_receive) = b.try_accept_uni_stream().unwrap().into_parts();
    let _pending_send = a.try_open_uni_stream(vec![2]).unwrap();

    a.close();

    assert_eq!(
        b.try_accept_uni_stream().unwrap_err(),
        StreamAcceptError::Closed
    );
    assert_eq!(
        active_receive.try_receive_chunk().unwrap_err(),
        StreamReceiveError::Cancelled
    );
    assert_eq!(
        active_send
            .try_send_chunk(vec![3])
            .unwrap_err()
            .into_chunk(),
        vec![3]
    );
    let closed_frame = vec![4];
    let error = match a.try_open_uni_stream(closed_frame.clone()) {
        Err(error) => error,
        Ok(_) => panic!("stream opened after connection close"),
    };
    assert_eq!(error.into_opening_frame(), closed_frame);
}

#[test]
fn empty_accept_is_nonblocking() {
    let pair = pair(1, 1, 8);
    let (_, b) = pair.endpoints();
    assert_eq!(
        b.try_accept_uni_stream().unwrap_err(),
        StreamAcceptError::Empty
    );
}

#[test]
fn stream_errors_have_distinct_semantics() {
    let full = StreamOpenError::Full(vec![1]);
    assert_eq!(full.into_opening_frame(), vec![1]);
    let too_large = StreamSendError::TooLarge(vec![2]);
    assert_eq!(too_large.into_chunk(), vec![2]);
}
