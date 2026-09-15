use std::sync::{Arc, Mutex, MutexGuard};

use crosslab_core::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, IncomingUniStream,
    StreamAcceptError, StreamOpenError, TransportConnection, TransportSecurityClass,
    TransportSendStream,
};
use iroh::endpoint::VarInt;
use tokio::sync::{Semaphore, mpsc};

use crate::{
    candidate::{
        control::{ControlBridge, connected_control_pair},
        endpoint::DirectPair,
        runtime::CandidateConfig,
        stream::{new_outgoing_uni_stream, run_outgoing_uni_stream, run_uni_acceptor},
    },
    error::EvalError,
};

const STREAM_FAILURE_CODE: VarInt = VarInt::from_u32(2);

pub(crate) struct IrohTransportConnection {
    control: ControlBridge,
    channel_binding: ChannelBinding,
    metadata: ConnectionMetadata,
    config: CandidateConfig,
    outgoing_stream_slots: Arc<Semaphore>,
    incoming_streams: Mutex<mpsc::Receiver<IncomingUniStream>>,
}

impl IrohTransportConnection {
    fn new(
        control: ControlBridge,
        channel_binding: ChannelBinding,
        metadata: ConnectionMetadata,
        config: CandidateConfig,
    ) -> Self {
        let (incoming_tx, incoming_streams) =
            mpsc::channel(config.incoming_stream_queue_capacity());
        let outgoing_stream_slots = Arc::new(Semaphore::new(config.outgoing_stream_capacity()));
        let incoming_stream_slots = Arc::new(Semaphore::new(
            config.max_concurrent_remote_uni_streams() as usize,
        ));
        let runtime = control.runtime();
        let spawned = runtime.spawn(run_uni_acceptor(
            runtime.connection(),
            runtime.shared(),
            incoming_tx,
            incoming_stream_slots,
            runtime.tasks(),
            config,
        ));
        debug_assert!(spawned, "new candidate runtime accepts its stream task");

        Self {
            control,
            channel_binding,
            metadata,
            config,
            outgoing_stream_slots,
            incoming_streams: Mutex::new(incoming_streams),
        }
    }

    pub(crate) async fn shutdown(self) {
        self.control.shutdown().await;
    }
}

impl TransportConnection for IrohTransportConnection {
    fn security_class(&self) -> TransportSecurityClass {
        TransportSecurityClass::AuthenticatedConfidentialChannel
    }

    fn channel_binding(&self) -> &ChannelBinding {
        &self.channel_binding
    }

    fn connection_metadata(&self) -> &ConnectionMetadata {
        &self.metadata
    }

    fn try_send_control(&self, frame: Vec<u8>) -> Result<(), ControlSendError> {
        self.control.try_send(frame)
    }

    fn try_receive_control(&self) -> Result<Vec<u8>, ControlReceiveError> {
        self.control.try_receive()
    }

    fn try_open_uni_stream(
        &self,
        opening_frame: Vec<u8>,
    ) -> Result<Box<dyn TransportSendStream>, StreamOpenError> {
        let runtime = self.control.runtime();
        if runtime.is_terminal() {
            return Err(StreamOpenError::Closed(opening_frame));
        }
        if opening_frame.len() > self.config.max_opening_frame_bytes() {
            return Err(StreamOpenError::TooLarge(opening_frame));
        }

        let permit = match Arc::clone(&self.outgoing_stream_slots).try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => return Err(StreamOpenError::Full(opening_frame)),
        };
        let connection = runtime.connection();
        let shared = runtime.shared();
        let chunk_capacity = self.config.stream_chunk_queue_capacity();
        let max_chunk_bytes = self.config.max_chunk_bytes();
        let mut opening_frame = Some(opening_frame);
        let mut permit = Some(permit);
        let mut handle = None;

        let spawned = runtime.spawn_if_open(|| {
            let (send, driver) =
                new_outgoing_uni_stream(Arc::clone(&shared), chunk_capacity, max_chunk_bytes);
            handle = Some(Box::new(send) as Box<dyn TransportSendStream>);
            run_outgoing_uni_stream(
                connection.clone(),
                Arc::clone(&shared),
                opening_frame
                    .take()
                    .expect("opening frame is available while spawning"),
                driver,
                permit
                    .take()
                    .expect("outgoing stream permit is available while spawning"),
            )
        });

        if !spawned {
            return Err(StreamOpenError::Closed(
                opening_frame.expect("opening frame remains when spawning is closed"),
            ));
        }
        Ok(handle.expect("send handle is created with its driver"))
    }

    fn try_accept_uni_stream(&self) -> Result<IncomingUniStream, StreamAcceptError> {
        let runtime = self.control.runtime();
        if runtime.is_terminal() {
            return Err(StreamAcceptError::Closed);
        }

        match lock(&self.incoming_streams).try_recv() {
            Ok(stream) => Ok(stream),
            Err(mpsc::error::TryRecvError::Empty) if runtime.is_terminal() => {
                Err(StreamAcceptError::Closed)
            }
            Err(mpsc::error::TryRecvError::Empty) => Err(StreamAcceptError::Empty),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                runtime.terminate(STREAM_FAILURE_CODE, b"stream bridge disconnected");
                Err(StreamAcceptError::Closed)
            }
        }
    }

    fn close(&self) {
        self.control.close();
    }

    fn is_closed(&self) -> bool {
        self.control.runtime().is_terminal()
    }
}

pub struct TransportPair {
    direct: DirectPair,
    client: IrohTransportConnection,
    server: IrohTransportConnection,
}

impl TransportPair {
    pub fn client(&self) -> &dyn TransportConnection {
        &self.client
    }

    pub fn server(&self) -> &dyn TransportConnection {
        &self.server
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

pub async fn connected_transport_pair(config: CandidateConfig) -> Result<TransportPair, EvalError> {
    let control_pair = connected_control_pair(config).await?;
    let (direct, client_control, server_control) = control_pair.into_parts();
    let client_binding = direct.client_binding().clone();
    let server_binding = direct.server_binding().clone();

    let client = IrohTransportConnection::new(
        client_control,
        client_binding,
        ConnectionMetadata::new(None, None, None),
        config,
    );
    let server = IrohTransportConnection::new(
        server_control,
        server_binding,
        ConnectionMetadata::new(None, None, None),
        config,
    );

    Ok(TransportPair {
        direct,
        client,
        server,
    })
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
