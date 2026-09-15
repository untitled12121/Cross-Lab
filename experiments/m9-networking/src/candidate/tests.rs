use std::sync::Arc;

use crosslab_core::{StreamSendError, TransportSendStream};

use super::{
    endpoint::direct_pair,
    runtime::CandidateRuntime,
    stream::new_outgoing_uni_stream,
};

#[tokio::test]
async fn outgoing_chunk_queue_reports_full_at_capacity() {
    let pair = direct_pair().await.expect("direct Iroh pair");
    let runtime = CandidateRuntime::new(pair.client_connection().clone());
    let (mut send, _driver) = new_outgoing_uni_stream(runtime.shared(), 1, 8);

    send.try_send_chunk(vec![0x01]).expect("first queued chunk");
    let second = vec![0x02];
    assert_eq!(
        send.try_send_chunk(second.clone()),
        Err(StreamSendError::Full(second))
    );

    runtime.shutdown().await;
    pair.shutdown().await;
}
