use std::sync::{Arc, Mutex, MutexGuard};

use crosslab_core::{ControlReceiveError, ControlSendError};
use iroh::endpoint::{Connection, RecvStream, SendStream, VarInt};
use tokio::sync::{mpsc, watch};

use crate::{
    candidate::{
        endpoint::{DirectPair, direct_pair},
        record::{read_record, write_record},
        runtime::{CandidateConfig, CandidateRuntime, SharedState, terminate_shared},
    },
    error::EvalError,
};

const CONTROL_FAILURE_CODE: VarInt = VarInt::from_u32(1);
const CONTROL_STREAM_MARKER: &[u8] = b"crosslab-m9-control-v1";

pub struct ControlBridge {
    runtime: CandidateRuntime,
    config: CandidateConfig,
    outbound: mpsc::Sender<Vec<u8>>,
    inbound: Mutex<mpsc::Receiver<Vec<u8>>>,
}

impl ControlBridge {
    fn new(
        connection: Connection,
        send: SendStream,
        recv: RecvStream,
        config: CandidateConfig,
    ) -> Self {
        let runtime = CandidateRuntime::new(connection);
        let (outbound, outbound_rx) = mpsc::channel(config.control_queue_capacity());
        let (inbound_tx, inbound) = mpsc::channel(config.control_queue_capacity());
        let max_control_frame_bytes = config.max_control_frame_bytes();

        runtime.spawn(run_writer(
            runtime.connection(),
            runtime.shared(),
            runtime.subscribe(),
            outbound_rx,
            send,
            max_control_frame_bytes,
        ));
        runtime.spawn(run_reader(
            runtime.connection(),
            runtime.shared(),
            runtime.subscribe(),
            inbound_tx,
            recv,
            max_control_frame_bytes,
        ));
        runtime.spawn(monitor_connection_close(
            runtime.connection(),
            runtime.shared(),
        ));

        Self {
            runtime,
            config,
            outbound,
            inbound: Mutex::new(inbound),
        }
    }

    pub fn try_send(&self, frame: Vec<u8>) -> Result<(), ControlSendError> {
        if self.runtime.is_terminal() {
            return Err(ControlSendError::Closed(frame));
        }
        if frame.len() > self.config.max_control_frame_bytes() {
            return Err(ControlSendError::TooLarge(frame));
        }

        match self.outbound.try_send(frame) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(frame)) => Err(ControlSendError::Full(frame)),
            Err(mpsc::error::TrySendError::Closed(frame)) => {
                self.runtime
                    .terminate(CONTROL_FAILURE_CODE, b"control bridge closed");
                Err(ControlSendError::Closed(frame))
            }
        }
    }

    pub fn try_receive(&self) -> Result<Vec<u8>, ControlReceiveError> {
        if self.runtime.is_terminal() {
            return Err(ControlReceiveError::Closed);
        }

        match lock(&self.inbound).try_recv() {
            Ok(frame) => Ok(frame),
            Err(mpsc::error::TryRecvError::Empty) if self.runtime.is_terminal() => {
                Err(ControlReceiveError::Closed)
            }
            Err(mpsc::error::TryRecvError::Empty) => Err(ControlReceiveError::Empty),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                self.runtime
                    .terminate(CONTROL_FAILURE_CODE, b"control bridge disconnected");
                Err(ControlReceiveError::Closed)
            }
        }
    }

    fn close(&self) {
        self.runtime.close();
    }

    pub async fn shutdown(self) {
        self.runtime.shutdown().await;
    }
}

pub struct ControlPair {
    direct: DirectPair,
    client: ControlBridge,
    server: ControlBridge,
}

impl ControlPair {
    pub fn client(&self) -> &ControlBridge {
        &self.client
    }

    pub fn server(&self) -> &ControlBridge {
        &self.server
    }

    pub fn close_server(&self) {
        self.server.close();
    }

    pub async fn shutdown(self) {
        let Self {
            direct,
            client,
            server,
        } = self;
        tokio::join!(client.shutdown(), server.shutdown());
        direct.shutdown().await;
    }
}

pub async fn connected_control_pair(config: CandidateConfig) -> Result<ControlPair, EvalError> {
    let direct = direct_pair().await?;
    let (mut client_send, client_recv) = direct
        .client_connection()
        .open_bi()
        .await
        .map_err(|_| EvalError::Control)?;

    write_record(
        &mut client_send,
        CONTROL_STREAM_MARKER,
        CONTROL_STREAM_MARKER.len(),
    )
    .await
    .map_err(|_| EvalError::Control)?;

    let (server_send, mut server_recv) = direct
        .server_connection()
        .accept_bi()
        .await
        .map_err(|_| EvalError::Control)?;
    let marker = read_record(&mut server_recv, CONTROL_STREAM_MARKER.len(), false)
        .await
        .map_err(|_| EvalError::Control)?;
    if marker != CONTROL_STREAM_MARKER {
        return Err(EvalError::Control);
    }

    let client = ControlBridge::new(
        direct.client_connection().clone(),
        client_send,
        client_recv,
        config,
    );
    let server = ControlBridge::new(
        direct.server_connection().clone(),
        server_send,
        server_recv,
        config,
    );

    Ok(ControlPair {
        direct,
        client,
        server,
    })
}

async fn run_writer(
    connection: Connection,
    shared: Arc<SharedState>,
    mut terminal: watch::Receiver<bool>,
    mut outbound: mpsc::Receiver<Vec<u8>>,
    mut send: SendStream,
    max_control_frame_bytes: usize,
) {
    loop {
        if *terminal.borrow() {
            return;
        }

        let frame = tokio::select! {
            changed = terminal.changed() => {
                if changed.is_err() || *terminal.borrow() {
                    return;
                }
                continue;
            }
            frame = outbound.recv() => frame,
        };

        let Some(frame) = frame else {
            terminate_shared(
                &connection,
                &shared,
                CONTROL_FAILURE_CODE,
                b"control sender dropped",
            );
            return;
        };
        if write_record(&mut send, &frame, max_control_frame_bytes)
            .await
            .is_err()
        {
            terminate_shared(
                &connection,
                &shared,
                CONTROL_FAILURE_CODE,
                b"control write failed",
            );
            return;
        }
    }
}

async fn run_reader(
    connection: Connection,
    shared: Arc<SharedState>,
    mut terminal: watch::Receiver<bool>,
    inbound: mpsc::Sender<Vec<u8>>,
    mut recv: RecvStream,
    max_control_frame_bytes: usize,
) {
    loop {
        if *terminal.borrow() {
            return;
        }

        let frame = tokio::select! {
            changed = terminal.changed() => {
                if changed.is_err() || *terminal.borrow() {
                    return;
                }
                continue;
            }
            frame = read_record(&mut recv, max_control_frame_bytes, false) => {
                match frame {
                    Ok(frame) => frame,
                    Err(_) => {
                        terminate_shared(
                            &connection,
                            &shared,
                            CONTROL_FAILURE_CODE,
                            b"control read failed",
                        );
                        return;
                    }
                }
            }
        };

        tokio::select! {
            changed = terminal.changed() => {
                if changed.is_err() || *terminal.borrow() {
                    return;
                }
            }
            result = inbound.send(frame) => {
                if result.is_err() {
                    terminate_shared(
                        &connection,
                        &shared,
                        CONTROL_FAILURE_CODE,
                        b"control receiver dropped",
                    );
                    return;
                }
            }
        }
    }
}

async fn monitor_connection_close(connection: Connection, shared: Arc<SharedState>) {
    let _ = connection.closed().await;
    shared.mark_terminal();
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
