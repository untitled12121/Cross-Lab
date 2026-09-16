use std::time::Duration;

use crosslab_core::{
    ControlReceiveError, IncomingUniStream, StreamAcceptError, StreamReceiveError,
    TransportConnection, TransportReceiveStream,
};
use crosslab_m9_networking::scenarios::{
    auth::AuthFixture,
    relay::{
        exercise_relay_operation_invariant, exercise_relay_sequence_invariant, relay_only_pair,
        relay_then_direct_pair,
    },
};
use crosslab_policy::NetworkClass;
use tokio::time::{sleep, timeout};

const WAIT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(5);

#[tokio::test]
async fn relay_only_pair_carries_authenticated_control_and_data() {
    let fixture = AuthFixture::new();
    let pair = relay_only_pair(&fixture)
        .await
        .expect("owner-relay authenticated pair");

    assert_eq!(pair.network_class(), NetworkClass::Remote);

    let control = b"owner-relay-control".to_vec();
    pair.client_transport()
        .try_send_control(control.clone())
        .expect("queue relay control frame");
    assert_eq!(
        eventually_receive_control(pair.server_transport()).await,
        control
    );

    let opening = b"owner-relay-stream".to_vec();
    let payload = b"owner-relay-payload".to_vec();
    let mut send = pair
        .client_transport()
        .try_open_uni_stream(opening.clone())
        .expect("open relay data stream");
    send.try_send_chunk(payload.clone())
        .expect("queue relay data chunk");
    send.finish();

    let incoming = eventually_accept(pair.server_transport()).await;
    assert_eq!(incoming.opening_frame(), opening);
    let (_, mut recv) = incoming.into_parts();
    assert_eq!(eventually_receive_chunk(recv.as_mut()).await, payload);
    eventually_finish(recv.as_mut()).await;

    pair.shutdown().await.expect("owner relay shutdown");
}

#[tokio::test]
async fn relay_to_direct_keeps_remote_class_binding_and_session() {
    let fixture = AuthFixture::new();
    let mut pair = relay_then_direct_pair(&fixture)
        .await
        .expect("relay-to-direct authenticated pair");
    let binding = pair.channel_binding().bytes().to_vec();
    let session_id = pair.session_id();

    assert_eq!(pair.network_class(), NetworkClass::Remote);
    pair.wait_for_direct_path(WAIT)
        .await
        .expect("direct path should become available");
    assert_eq!(pair.network_class(), NetworkClass::Remote);
    assert_eq!(binding, pair.channel_binding().bytes());
    assert_eq!(session_id, pair.session_id());

    pair.shutdown().await.expect("owner relay shutdown");
}

#[tokio::test]
async fn relay_to_direct_keeps_control_sequence_space() {
    let evidence = exercise_relay_sequence_invariant(&AuthFixture::new())
        .await
        .expect("relay path sequence evidence");

    assert_eq!(evidence.before_send_sequence, Some(1));
    assert_eq!(evidence.before_receive_sequence, Some(1));
    assert_eq!(evidence.after_send_sequence, Some(2));
    assert_eq!(evidence.after_receive_sequence, Some(2));
}

#[tokio::test]
async fn relay_to_direct_keeps_active_operation_authority() {
    let payload = exercise_relay_operation_invariant(&AuthFixture::new())
        .await
        .expect("relay path operation evidence");

    assert_eq!(payload, b"path-operation");
}

async fn eventually_receive_control(transport: &dyn TransportConnection) -> Vec<u8> {
    timeout(WAIT, async {
        loop {
            match transport.try_receive_control() {
                Ok(frame) => return frame,
                Err(ControlReceiveError::Empty) => sleep(POLL).await,
                Err(error) => panic!("control receive failed: {error:?}"),
            }
        }
    })
    .await
    .expect("control frame should arrive")
}

async fn eventually_accept(transport: &dyn TransportConnection) -> IncomingUniStream {
    timeout(WAIT, async {
        loop {
            match transport.try_accept_uni_stream() {
                Ok(stream) => return stream,
                Err(StreamAcceptError::Empty) => sleep(POLL).await,
                Err(error) => panic!("stream accept failed: {error:?}"),
            }
        }
    })
    .await
    .expect("data stream should arrive")
}

async fn eventually_receive_chunk(stream: &mut dyn TransportReceiveStream) -> Vec<u8> {
    timeout(WAIT, async {
        loop {
            match stream.try_receive_chunk() {
                Ok(chunk) => return chunk,
                Err(StreamReceiveError::Empty) => sleep(POLL).await,
                Err(error) => panic!("stream receive failed: {error:?}"),
            }
        }
    })
    .await
    .expect("data chunk should arrive")
}

async fn eventually_finish(stream: &mut dyn TransportReceiveStream) {
    timeout(WAIT, async {
        loop {
            match stream.try_receive_chunk() {
                Err(StreamReceiveError::Finished) => return,
                Err(StreamReceiveError::Empty) => sleep(POLL).await,
                Ok(chunk) => panic!("unexpected extra stream chunk: {} bytes", chunk.len()),
                Err(error) => panic!("stream finish failed: {error:?}"),
            }
        }
    })
    .await
    .expect("data stream should finish");
}
