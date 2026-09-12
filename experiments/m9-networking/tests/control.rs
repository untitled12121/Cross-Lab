use std::{num::NonZeroUsize, time::Duration};

use crosslab_core::{ControlReceiveError, ControlSendError};
use crosslab_m9_networking::candidate::{
    control::connected_control_pair,
    endpoint::direct_pair,
    record::{RecordError, read_record, write_record},
    runtime::CandidateConfig,
};
use crosslab_transport_quic::QuicTransportConfig;
use tokio::time::timeout;

#[test]
fn candidate_defaults_match_quinn_semantic_limits() {
    let candidate = CandidateConfig::default();
    let quinn = QuicTransportConfig::default();

    assert_eq!(candidate.control_queue_capacity(), quinn.control_queue_capacity());
    assert_eq!(
        candidate.incoming_stream_queue_capacity(),
        quinn.incoming_stream_queue_capacity()
    );
    assert_eq!(
        candidate.outgoing_stream_capacity(),
        quinn.outgoing_stream_capacity()
    );
    assert_eq!(
        candidate.stream_chunk_queue_capacity(),
        quinn.stream_chunk_queue_capacity()
    );
    assert_eq!(
        candidate.max_control_frame_bytes(),
        quinn.max_control_frame_bytes()
    );
    assert_eq!(
        candidate.max_opening_frame_bytes(),
        quinn.max_opening_frame_bytes()
    );
    assert_eq!(candidate.max_chunk_bytes(), quinn.max_chunk_bytes());
    assert_eq!(
        candidate.max_concurrent_remote_uni_streams(),
        quinn.max_concurrent_remote_uni_streams()
    );
    assert_eq!(
        candidate.max_concurrent_remote_bi_streams(),
        quinn.max_concurrent_remote_bi_streams()
    );
    assert_eq!(candidate.stream_receive_window(), quinn.stream_receive_window());
    assert_eq!(
        candidate.connection_receive_window(),
        quinn.connection_receive_window()
    );
    assert_eq!(candidate.idle_timeout(), quinn.idle_timeout());
}

#[tokio::test]
async fn record_round_trip_accepts_exact_limit() {
    let pair = direct_pair().await.expect("direct Iroh pair");
    let (mut send, _) = pair
        .client_connection()
        .open_bi()
        .await
        .expect("open raw record stream");
    let payload = vec![0x41; 8];

    write_record(&mut send, &payload, 8)
        .await
        .expect("write exact-limit record");
    send.finish().expect("finish raw record stream");
    let (_, mut recv) = pair
        .server_connection()
        .accept_bi()
        .await
        .expect("accept raw record stream");

    assert_eq!(
        read_record(&mut recv, 8, false)
            .await
            .expect("read exact-limit record"),
        payload
    );
    pair.shutdown().await;
}

#[tokio::test]
async fn record_reader_rejects_declared_length_before_allocating_body() {
    let pair = direct_pair().await.expect("direct Iroh pair");
    let (mut send, _) = pair
        .client_connection()
        .open_bi()
        .await
        .expect("open malformed record stream");
    send.write_all(&9_u32.to_be_bytes())
        .await
        .expect("write declared length");
    send.write_all(&[0_u8])
        .await
        .expect("write partial body");
    send.finish().expect("finish malformed record stream");
    let (_, mut recv) = pair
        .server_connection()
        .accept_bi()
        .await
        .expect("accept malformed record stream");

    assert_eq!(
        read_record(&mut recv, 8, false).await,
        Err(RecordError::TooLarge {
            declared: 9,
            max: 8,
        })
    );
    pair.shutdown().await;
}

#[tokio::test]
async fn record_reader_rejects_truncated_body() {
    let pair = direct_pair().await.expect("direct Iroh pair");
    let (mut send, _) = pair
        .client_connection()
        .open_bi()
        .await
        .expect("open truncated record stream");
    send.write_all(&4_u32.to_be_bytes())
        .await
        .expect("write declared length");
    send.write_all(&[0x51, 0x52])
        .await
        .expect("write truncated body");
    send.finish().expect("finish truncated record stream");
    let (_, mut recv) = pair
        .server_connection()
        .accept_bi()
        .await
        .expect("accept truncated record stream");

    assert_eq!(read_record(&mut recv, 8, false).await, Err(RecordError::Read));
    pair.shutdown().await;
}

#[tokio::test]
async fn oversized_control_returns_original_frame_without_queueing() {
    let config = CandidateConfig::default().with_control_limits(nonzero(1), nonzero(4));
    let pair = connected_control_pair(config)
        .await
        .expect("connected control pair");
    let frame = vec![0x55; 5];

    assert_eq!(
        pair.client().try_send(frame.clone()),
        Err(ControlSendError::TooLarge(frame))
    );
    assert_eq!(pair.server().try_receive(), Err(ControlReceiveError::Empty));

    let exact = vec![0x66; 4];
    pair.client()
        .try_send(exact.clone())
        .expect("exact-limit control frame");
    assert_eq!(eventually_receive(pair.server()).await, exact);
    pair.shutdown().await;
}

#[tokio::test]
async fn control_bridge_preserves_order_and_bounded_backpressure() {
    let config = CandidateConfig::default().with_control_limits(nonzero(1), nonzero(8));
    let pair = connected_control_pair(config)
        .await
        .expect("connected control pair");

    assert_eq!(pair.server().try_receive(), Err(ControlReceiveError::Empty));
    pair.client().try_send(vec![1]).expect("first queued frame");
    assert_eq!(
        pair.client().try_send(vec![2]),
        Err(ControlSendError::Full(vec![2]))
    );

    assert_eq!(eventually_receive(pair.server()).await, vec![1]);
    pair.client().try_send(vec![2]).expect("second queued frame");
    assert_eq!(eventually_receive(pair.server()).await, vec![2]);
    pair.shutdown().await;
}

#[tokio::test]
async fn peer_close_becomes_closed_without_blocking() {
    let pair = connected_control_pair(CandidateConfig::default())
        .await
        .expect("connected control pair");

    pair.close_server();
    eventually_closed(pair.client()).await;

    let frame = vec![0x71];
    assert_eq!(
        pair.client().try_send(frame.clone()),
        Err(ControlSendError::Closed(frame))
    );
    assert_eq!(
        pair.client().try_receive(),
        Err(ControlReceiveError::Closed)
    );
    pair.shutdown().await;
}

#[tokio::test]
async fn shutdown_joins_owned_control_tasks() {
    let pair = connected_control_pair(CandidateConfig::default())
        .await
        .expect("connected control pair");

    timeout(Duration::from_secs(5), pair.shutdown())
        .await
        .expect("control shutdown timed out");
}

fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).expect("test limit must be nonzero")
}

async fn eventually_receive(
    bridge: &crosslab_m9_networking::candidate::control::ControlBridge,
) -> Vec<u8> {
    timeout(Duration::from_secs(5), async {
        loop {
            match bridge.try_receive() {
                Ok(frame) => return frame,
                Err(ControlReceiveError::Empty) => tokio::task::yield_now().await,
                Err(ControlReceiveError::Closed) => panic!("control bridge closed before receive"),
            }
        }
    })
    .await
    .expect("control receive timed out")
}

async fn eventually_closed(
    bridge: &crosslab_m9_networking::candidate::control::ControlBridge,
) {
    timeout(Duration::from_secs(5), async {
        loop {
            if bridge.try_receive() == Err(ControlReceiveError::Closed) {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("control bridge did not become closed");
}
