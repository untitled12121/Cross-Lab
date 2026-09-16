use std::time::Duration;

use crosslab_core::{
    ControlReceiveError, StreamAcceptError, StreamReceiveError, TransportConnection,
};
use crosslab_m9_networking::scenarios::auth::{
    AuthAttempt, AuthFixture, authenticate_direct_pair,
};
use tokio::{task::yield_now, time::timeout};

const PHASE_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn task8_direct_benchmark_phases_complete_independently() {
    let fixture = AuthFixture::new();
    let pair = timeout(
        PHASE_TIMEOUT,
        authenticate_direct_pair(&fixture, AuthAttempt::Normal),
    )
    .await
    .expect("Task 8 diagnostic: authentication timed out")
    .expect("Task 8 diagnostic: authentication failed");
    eprintln!("task8 diagnostic: auth complete");

    timeout(
        PHASE_TIMEOUT,
        control_round_trip(pair.client_transport(), pair.server_transport()),
    )
    .await
    .expect("Task 8 diagnostic: control timed out");
    eprintln!("task8 diagnostic: control complete");

    timeout(
        PHASE_TIMEOUT,
        one_chunk_transfer(pair.client_transport(), pair.server_transport()),
    )
    .await
    .expect("Task 8 diagnostic: bulk timed out");
    eprintln!("task8 diagnostic: bulk complete");

    timeout(PHASE_TIMEOUT, pair.shutdown())
        .await
        .expect("Task 8 diagnostic: shutdown timed out");
    eprintln!("task8 diagnostic: shutdown complete");
}

async fn control_round_trip(client: &dyn TransportConnection, server: &dyn TransportConnection) {
    let frame = b"task8-diagnostic-control".to_vec();
    client
        .try_send_control(frame.clone())
        .expect("Task 8 diagnostic: send control");
    let received = eventually_control(server).await;
    assert_eq!(received, frame);
    server
        .try_send_control(received)
        .expect("Task 8 diagnostic: echo control");
    assert_eq!(eventually_control(client).await, frame);
}

async fn eventually_control(transport: &dyn TransportConnection) -> Vec<u8> {
    loop {
        match transport.try_receive_control() {
            Ok(frame) => return frame,
            Err(ControlReceiveError::Empty) => yield_now().await,
            Err(error) => panic!("Task 8 diagnostic: control failed: {error:?}"),
        }
    }
}

async fn one_chunk_transfer(client: &dyn TransportConnection, server: &dyn TransportConnection) {
    let opening = b"task8-diagnostic-stream".to_vec();
    let payload = vec![0x5A; 64 * 1024];

    let sender = async {
        let mut send = client
            .try_open_uni_stream(opening.clone())
            .expect("Task 8 diagnostic: open uni");
        send.try_send_chunk(payload.clone())
            .expect("Task 8 diagnostic: send chunk");
        send.finish();
    };

    let receiver = async {
        let incoming = loop {
            match server.try_accept_uni_stream() {
                Ok(stream) => break stream,
                Err(StreamAcceptError::Empty) => yield_now().await,
                Err(error) => panic!("Task 8 diagnostic: accept failed: {error:?}"),
            }
        };
        assert_eq!(incoming.opening_frame(), opening);
        let (_, mut recv) = incoming.into_parts();
        let received = loop {
            match recv.try_receive_chunk() {
                Ok(chunk) => break chunk,
                Err(StreamReceiveError::Empty) => yield_now().await,
                Err(error) => panic!("Task 8 diagnostic: receive failed: {error:?}"),
            }
        };
        assert_eq!(received, payload);
        loop {
            match recv.try_receive_chunk() {
                Err(StreamReceiveError::Finished) => break,
                Err(StreamReceiveError::Empty) => yield_now().await,
                Ok(extra) => panic!("Task 8 diagnostic: extra {} byte chunk", extra.len()),
                Err(error) => panic!("Task 8 diagnostic: finish failed: {error:?}"),
            }
        }
    };

    tokio::join!(sender, receiver);
}
