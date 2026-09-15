use std::time::Duration;

use crosslab_core::{
    IncomingUniStream, StreamAcceptError, StreamOpenError, StreamReceiveError, StreamSendError,
    TransportConnection, TransportReceiveStream, TransportSecurityClass,
};
use crosslab_m9_networking::candidate::{
    connection::{IrohTransportConnection, connected_transport_pair},
    runtime::CandidateConfig,
};
use tokio::time::timeout;

#[tokio::test]
async fn transport_surface_reuses_exporter_and_authenticated_security_class() {
    let pair = connected_transport_pair(CandidateConfig::default())
        .await
        .expect("connected Iroh transport pair");

    assert_eq!(
        pair.client().security_class(),
        TransportSecurityClass::AuthenticatedConfidentialChannel
    );
    assert_eq!(
        pair.client().channel_binding(),
        pair.server().channel_binding()
    );
    assert_eq!(
        pair.client().channel_binding().profile_id(),
        "quic-tls-exporter-v1"
    );
    assert_eq!(pair.client().channel_binding().bytes().len(), 32);

    pair.shutdown().await;
}

#[tokio::test]
async fn uni_stream_carries_opening_and_chunks_in_order_then_finishes() {
    let pair = connected_transport_pair(CandidateConfig::default())
        .await
        .expect("connected Iroh transport pair");
    let mut send = pair.client().try_open_uni_stream(vec![0x10]).unwrap();

    send.try_send_chunk(vec![0x20]).unwrap();
    send.try_send_chunk(vec![0x21]).unwrap();
    send.finish();

    let incoming = eventually_accept(pair.server()).await;
    assert_eq!(incoming.opening_frame(), &[0x10]);
    let (_, mut recv) = incoming.into_parts();
    assert_eq!(eventually_receive_chunk(recv.as_mut()).await, vec![0x20]);
    assert_eq!(eventually_receive_chunk(recv.as_mut()).await, vec![0x21]);
    eventually_finished(recv.as_mut()).await;

    pair.shutdown().await;
}

#[tokio::test]
async fn uni_stream_enforces_opening_and_chunk_limits_without_consuming_slot() {
    let config = CandidateConfig::default();
    let pair = connected_transport_pair(config)
        .await
        .expect("connected Iroh transport pair");

    let opening = vec![0x31; config.max_opening_frame_bytes() + 1];
    let error = match pair.client().try_open_uni_stream(opening.clone()) {
        Err(error) => error,
        Ok(_) => panic!("oversized opening unexpectedly allocated a stream"),
    };
    assert_eq!(error, StreamOpenError::TooLarge(opening));

    let mut send = pair.client().try_open_uni_stream(vec![0x32]).unwrap();
    let oversized = vec![0x41; config.max_chunk_bytes() + 1];
    assert_eq!(
        send.try_send_chunk(oversized.clone()).unwrap_err(),
        StreamSendError::TooLarge(oversized)
    );

    send.cancel();
    pair.shutdown().await;
}

#[tokio::test]
async fn uni_stream_open_and_chunk_queues_apply_bounded_backpressure() {
    let config = CandidateConfig::default();
    let pair = connected_transport_pair(config)
        .await
        .expect("connected Iroh transport pair");
    let mut streams = Vec::with_capacity(config.outgoing_stream_capacity());

    for index in 0..config.outgoing_stream_capacity() {
        streams.push(
            pair.client()
                .try_open_uni_stream(vec![u8::try_from(index).unwrap()])
                .unwrap(),
        );
    }

    let opening = vec![0xF1];
    let error = match pair.client().try_open_uni_stream(opening.clone()) {
        Err(error) => error,
        Ok(_) => panic!("outgoing stream capacity was not enforced"),
    };
    assert_eq!(error, StreamOpenError::Full(opening));

    let first = streams.first_mut().expect("at least one outgoing stream");
    for index in 0..config.stream_chunk_queue_capacity() {
        first
            .try_send_chunk(vec![u8::try_from(index).unwrap()])
            .unwrap();
    }
    let full = vec![0xF2];
    assert_eq!(
        first.try_send_chunk(full.clone()).unwrap_err(),
        StreamSendError::Full(full)
    );

    for stream in &mut streams {
        stream.cancel();
    }
    pair.shutdown().await;
}

#[tokio::test]
async fn sender_cancel_and_drop_cancel_receiver() {
    let pair = connected_transport_pair(CandidateConfig::default())
        .await
        .expect("connected Iroh transport pair");

    let mut cancelled_send = pair.client().try_open_uni_stream(vec![0x50]).unwrap();
    let cancelled_incoming = eventually_accept(pair.server()).await;
    let (_, mut cancelled_recv) = cancelled_incoming.into_parts();
    cancelled_send.cancel();
    eventually_cancelled(cancelled_recv.as_mut()).await;

    let dropped_send = pair.client().try_open_uni_stream(vec![0x51]).unwrap();
    let dropped_incoming = eventually_accept(pair.server()).await;
    let (_, mut dropped_recv) = dropped_incoming.into_parts();
    drop(dropped_send);
    eventually_cancelled(dropped_recv.as_mut()).await;

    pair.shutdown().await;
}

#[tokio::test]
async fn receiver_cancel_closes_sender() {
    let pair = connected_transport_pair(CandidateConfig::default())
        .await
        .expect("connected Iroh transport pair");
    let mut send = pair.client().try_open_uni_stream(vec![0x60]).unwrap();
    let incoming = eventually_accept(pair.server()).await;
    let (_, mut recv) = incoming.into_parts();

    recv.cancel();
    eventually_sender_closed(send.as_mut()).await;

    pair.shutdown().await;
}

#[tokio::test]
async fn connection_close_cancels_live_and_future_streams() {
    let pair = connected_transport_pair(CandidateConfig::default())
        .await
        .expect("connected Iroh transport pair");
    let mut send = pair.client().try_open_uni_stream(vec![0x70]).unwrap();
    let incoming = eventually_accept(pair.server()).await;
    let (_, mut recv) = incoming.into_parts();

    pair.server().close();
    eventually_closed(pair.client()).await;
    eventually_cancelled(recv.as_mut()).await;

    assert_eq!(
        send.try_send_chunk(vec![0x71]).unwrap_err(),
        StreamSendError::Closed(vec![0x71])
    );
    let opening = vec![0x72];
    let error = match pair.client().try_open_uni_stream(opening.clone()) {
        Err(error) => error,
        Ok(_) => panic!("stream opened after connection became terminal"),
    };
    assert_eq!(error, StreamOpenError::Closed(opening));
    assert_eq!(
        pair.server().try_accept_uni_stream().unwrap_err(),
        StreamAcceptError::Closed
    );

    pair.shutdown().await;
}

#[tokio::test]
async fn shutdown_joins_owned_stream_tasks() {
    let pair = connected_transport_pair(CandidateConfig::default())
        .await
        .expect("connected Iroh transport pair");
    let mut send = pair.client().try_open_uni_stream(vec![0x80]).unwrap();
    let incoming = eventually_accept(pair.server()).await;
    let (_, mut recv) = incoming.into_parts();
    send.try_send_chunk(vec![0x81]).unwrap();

    timeout(Duration::from_secs(5), pair.shutdown())
        .await
        .expect("Iroh transport shutdown timed out");
    assert_eq!(
        send.try_send_chunk(vec![0x82]),
        Err(StreamSendError::Closed(vec![0x82]))
    );
    eventually_cancelled(recv.as_mut()).await;
}

async fn eventually_accept(connection: &IrohTransportConnection) -> IncomingUniStream {
    timeout(Duration::from_secs(5), async {
        loop {
            match connection.try_accept_uni_stream() {
                Ok(stream) => return stream,
                Err(StreamAcceptError::Empty) => tokio::task::yield_now().await,
                Err(StreamAcceptError::Closed) => {
                    panic!("transport closed before incoming stream was published")
                }
            }
        }
    })
    .await
    .expect("incoming stream was not published before timeout")
}

async fn eventually_receive_chunk(stream: &mut dyn TransportReceiveStream) -> Vec<u8> {
    timeout(Duration::from_secs(5), async {
        loop {
            match stream.try_receive_chunk() {
                Ok(chunk) => return chunk,
                Err(StreamReceiveError::Empty) => tokio::task::yield_now().await,
                Err(other) => panic!("stream ended before chunk arrived: {other:?}"),
            }
        }
    })
    .await
    .expect("stream chunk was not delivered before timeout")
}

async fn eventually_finished(stream: &mut dyn TransportReceiveStream) {
    timeout(Duration::from_secs(5), async {
        loop {
            match stream.try_receive_chunk() {
                Err(StreamReceiveError::Finished) => return,
                Err(StreamReceiveError::Empty) => tokio::task::yield_now().await,
                Ok(chunk) => panic!("unexpected chunk after expected finish: {chunk:?}"),
                Err(other) => panic!("stream did not finish cleanly: {other:?}"),
            }
        }
    })
    .await
    .expect("stream did not finish before timeout");
}

async fn eventually_cancelled(stream: &mut dyn TransportReceiveStream) {
    timeout(Duration::from_secs(5), async {
        loop {
            match stream.try_receive_chunk() {
                Err(StreamReceiveError::Cancelled) => return,
                Err(StreamReceiveError::Empty) => tokio::task::yield_now().await,
                Ok(_) | Err(StreamReceiveError::Finished) => {
                    panic!("stream was not cancelled")
                }
            }
        }
    })
    .await
    .expect("stream was not cancelled before timeout");
}

async fn eventually_sender_closed(stream: &mut dyn crosslab_core::TransportSendStream) {
    timeout(Duration::from_secs(5), async {
        loop {
            let chunk = vec![0x61];
            match stream.try_send_chunk(chunk.clone()) {
                Err(StreamSendError::Closed(returned)) => {
                    assert_eq!(returned, chunk);
                    return;
                }
                Err(StreamSendError::Full(_)) | Ok(()) => tokio::task::yield_now().await,
                Err(other) => panic!("unexpected sender state: {other:?}"),
            }
        }
    })
    .await
    .expect("sender did not observe receiver stop before timeout");
}

async fn eventually_closed(connection: &IrohTransportConnection) {
    timeout(Duration::from_secs(5), async {
        loop {
            if connection.is_closed() {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("transport did not become closed before timeout");
}
