use core::fmt;
use std::num::NonZeroUsize;

use crosslab_core::{
    AdmittedStream, LogicalSession, SessionError, SessionState, StreamAcceptError, StreamAdmission,
    StreamAdmissionError, StreamOpenError, StreamReceiveError, StreamSendError,
    TransportConnection, TransportReceiveStream, TransportSendStream,
};
use crosslab_policy::{AuthorizedOperation, PolicyState, TrustRecord};
use crosslab_protocol::{
    DataStreamOpen, ProtocolWireError, StreamId, decode_data_stream_open, encode_data_stream_open,
};

#[derive(Debug)]
pub enum RuntimeStreamError {
    Session(SessionError),
    Wire(ProtocolWireError),
    Admission(StreamAdmissionError),
    Open(StreamOpenError),
    Accept(StreamAcceptError),
    Receive(StreamReceiveError),
    StreamNotFound,
    DuplicateStream,
    ResourceLimit,
}

impl fmt::Display for RuntimeStreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(error) => fmt::Display::fmt(error, formatter),
            Self::Wire(error) => fmt::Display::fmt(error, formatter),
            Self::Admission(error) => fmt::Display::fmt(error, formatter),
            Self::Open(error) => fmt::Display::fmt(error, formatter),
            Self::Accept(error) => fmt::Display::fmt(error, formatter),
            Self::Receive(error) => fmt::Display::fmt(error, formatter),
            Self::StreamNotFound => formatter.write_str("runtime data stream was not found"),
            Self::DuplicateStream => {
                formatter.write_str("runtime data stream identifier is already active")
            }
            Self::ResourceLimit => formatter.write_str("runtime data stream capacity is exhausted"),
        }
    }
}

impl std::error::Error for RuntimeStreamError {}

impl From<SessionError> for RuntimeStreamError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

impl From<ProtocolWireError> for RuntimeStreamError {
    fn from(error: ProtocolWireError) -> Self {
        Self::Wire(error)
    }
}

impl From<StreamAdmissionError> for RuntimeStreamError {
    fn from(error: StreamAdmissionError) -> Self {
        Self::Admission(error)
    }
}

impl From<StreamOpenError> for RuntimeStreamError {
    fn from(error: StreamOpenError) -> Self {
        Self::Open(error)
    }
}

impl From<StreamAcceptError> for RuntimeStreamError {
    fn from(error: StreamAcceptError) -> Self {
        Self::Accept(error)
    }
}

impl From<StreamReceiveError> for RuntimeStreamError {
    fn from(error: StreamReceiveError) -> Self {
        Self::Receive(error)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RuntimeStreamChunk {
    stream_id: StreamId,
    bytes: Vec<u8>,
}

impl RuntimeStreamChunk {
    fn new(stream_id: StreamId, bytes: Vec<u8>) -> Self {
        Self { stream_id, bytes }
    }

    pub const fn stream_id(&self) -> StreamId {
        self.stream_id
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl fmt::Debug for RuntimeStreamChunk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeStreamChunk")
            .field("stream_id", &self.stream_id)
            .field(
                "bytes",
                &format_args!("[REDACTED; {} bytes]", self.bytes.len()),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeStreamEvent {
    Opened(AdmittedStream),
    Chunk(RuntimeStreamChunk),
    Finished(StreamId),
    Cancelled(StreamId),
}

struct InboundStream {
    admitted: AdmittedStream,
    stream: Box<dyn TransportReceiveStream>,
}

struct OutboundStream {
    stream_id: StreamId,
    stream: Box<dyn TransportSendStream>,
}

pub(crate) struct RuntimeStreams {
    admission: StreamAdmission,
    capacity: usize,
    inbound: Vec<InboundStream>,
    outbound: Vec<OutboundStream>,
}

impl RuntimeStreams {
    pub(crate) fn new(capacity: NonZeroUsize) -> Self {
        Self {
            admission: StreamAdmission::new(capacity),
            capacity: capacity.get(),
            inbound: Vec::with_capacity(capacity.get()),
            outbound: Vec::with_capacity(capacity.get()),
        }
    }

    pub(crate) fn register_operation(
        &mut self,
        session: &LogicalSession,
        operation: AuthorizedOperation,
    ) -> Result<(), RuntimeStreamError> {
        ensure_active(session)?;
        self.admission.register_operation(operation)?;
        Ok(())
    }

    pub(crate) fn open_uni(
        &mut self,
        session: &LogicalSession,
        transport: &dyn TransportConnection,
        open: &DataStreamOpen,
    ) -> Result<StreamId, RuntimeStreamError> {
        ensure_active(session)?;
        validate_open_for_session(session, open)?;

        if self.outbound.len() >= self.capacity {
            return Err(RuntimeStreamError::ResourceLimit);
        }
        if self
            .outbound
            .iter()
            .any(|outbound| outbound.stream_id == open.stream_id())
        {
            return Err(RuntimeStreamError::DuplicateStream);
        }

        let frame = encode_data_stream_open(open)?;
        let stream = transport.try_open_uni_stream(frame)?;
        let stream_id = open.stream_id();
        self.outbound.push(OutboundStream { stream_id, stream });
        Ok(stream_id)
    }

    pub(crate) fn try_send_chunk(
        &mut self,
        stream_id: StreamId,
        chunk: Vec<u8>,
    ) -> Result<(), StreamSendError> {
        let Some(position) = self
            .outbound
            .iter()
            .position(|outbound| outbound.stream_id == stream_id)
        else {
            return Err(StreamSendError::Closed(chunk));
        };

        let result = self.outbound[position].stream.try_send_chunk(chunk);
        if matches!(&result, Err(StreamSendError::Closed(_))) {
            self.outbound.remove(position);
        }
        result
    }

    pub(crate) fn finish_outbound(
        &mut self,
        stream_id: StreamId,
    ) -> Result<(), RuntimeStreamError> {
        let position = self
            .outbound
            .iter()
            .position(|outbound| outbound.stream_id == stream_id)
            .ok_or(RuntimeStreamError::StreamNotFound)?;
        let mut outbound = self.outbound.remove(position);
        outbound.stream.finish();
        Ok(())
    }

    pub(crate) fn cancel_outbound(
        &mut self,
        stream_id: StreamId,
    ) -> Result<(), RuntimeStreamError> {
        let position = self
            .outbound
            .iter()
            .position(|outbound| outbound.stream_id == stream_id)
            .ok_or(RuntimeStreamError::StreamNotFound)?;
        let mut outbound = self.outbound.remove(position);
        outbound.stream.cancel();
        Ok(())
    }

    pub(crate) fn receive_one(
        &mut self,
        session: &LogicalSession,
        transport: &dyn TransportConnection,
        now: u64,
        peer_trust: &TrustRecord,
        policy: &PolicyState,
    ) -> Result<RuntimeStreamEvent, RuntimeStreamError> {
        ensure_active(session)?;

        match self.accept_one(session, transport, now, peer_trust, policy) {
            Ok(event) => return Ok(event),
            Err(RuntimeStreamError::Accept(StreamAcceptError::Empty)) => {}
            Err(error) => return Err(error),
        }

        for index in 0..self.inbound.len() {
            let stream_id = self.inbound[index].admitted.stream_id();
            match self.inbound[index].stream.try_receive_chunk() {
                Ok(bytes) => {
                    return Ok(RuntimeStreamEvent::Chunk(RuntimeStreamChunk::new(
                        stream_id, bytes,
                    )));
                }
                Err(StreamReceiveError::Empty) => {}
                Err(StreamReceiveError::Finished) => {
                    self.admission.finish_stream(stream_id)?;
                    self.inbound.remove(index);
                    return Ok(RuntimeStreamEvent::Finished(stream_id));
                }
                Err(StreamReceiveError::Cancelled) => {
                    self.admission.cancel_stream(stream_id)?;
                    self.inbound.remove(index);
                    return Ok(RuntimeStreamEvent::Cancelled(stream_id));
                }
            }
        }

        Err(RuntimeStreamError::Receive(StreamReceiveError::Empty))
    }

    pub(crate) fn cancel_inbound(&mut self, stream_id: StreamId) -> Result<(), RuntimeStreamError> {
        let position = self
            .inbound
            .iter()
            .position(|inbound| inbound.admitted.stream_id() == stream_id)
            .ok_or(RuntimeStreamError::StreamNotFound)?;
        let mut inbound = self.inbound.remove(position);
        inbound.stream.cancel();
        self.admission.cancel_stream(stream_id)?;
        Ok(())
    }

    pub(crate) fn cancel_all(&mut self) {
        for inbound in &mut self.inbound {
            inbound.stream.cancel();
        }
        self.inbound.clear();

        for outbound in &mut self.outbound {
            outbound.stream.cancel();
        }
        self.outbound.clear();

        self.admission.cancel_all();
    }

    fn accept_one(
        &mut self,
        session: &LogicalSession,
        transport: &dyn TransportConnection,
        now: u64,
        peer_trust: &TrustRecord,
        policy: &PolicyState,
    ) -> Result<RuntimeStreamEvent, RuntimeStreamError> {
        let incoming = transport.try_accept_uni_stream()?;
        let (frame, mut stream) = incoming.into_parts();
        if self.inbound.len() >= self.capacity {
            stream.cancel();
            return Err(RuntimeStreamError::ResourceLimit);
        }
        let open = match decode_data_stream_open(&frame) {
            Ok(open) => open,
            Err(error) => {
                stream.cancel();
                return Err(error.into());
            }
        };
        let admitted = match self
            .admission
            .admit_inbound(session, &open, now, peer_trust, policy)
        {
            Ok(admitted) => admitted,
            Err(error) => {
                stream.cancel();
                return Err(error.into());
            }
        };

        self.inbound.push(InboundStream { admitted, stream });
        Ok(RuntimeStreamEvent::Opened(admitted))
    }
}

fn ensure_active(session: &LogicalSession) -> Result<(), RuntimeStreamError> {
    if session.state() == SessionState::Active {
        Ok(())
    } else {
        Err(RuntimeStreamError::Session(SessionError::InvalidState))
    }
}

fn validate_open_for_session(
    session: &LogicalSession,
    open: &DataStreamOpen,
) -> Result<(), RuntimeStreamError> {
    let context = session
        .context()
        .ok_or(RuntimeStreamError::Session(SessionError::InvalidState))?;
    if open.session_id() != context.session_id() {
        return Err(RuntimeStreamError::Admission(
            StreamAdmissionError::InvalidSession,
        ));
    }
    if !context.negotiated_capabilities().iter().any(|capability| {
        capability.capability_id() == open.capability_id()
            && capability.version() == open.capability_version()
    }) {
        return Err(RuntimeStreamError::Admission(
            StreamAdmissionError::CapabilityNotNegotiated,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ClosedSendStream;

    impl TransportSendStream for ClosedSendStream {
        fn try_send_chunk(&mut self, chunk: Vec<u8>) -> Result<(), StreamSendError> {
            Err(StreamSendError::Closed(chunk))
        }

        fn finish(&mut self) {}

        fn cancel(&mut self) {}
    }

    #[test]
    fn chunk_debug_redacts_payload() {
        let chunk = RuntimeStreamChunk::new(
            StreamId::from_bytes([0x51; 16]),
            b"private-file-bytes".to_vec(),
        );
        let debug = format!("{chunk:?}");

        assert!(!debug.contains("private-file-bytes"));
        assert!(debug.contains("18 bytes"));
    }

    #[test]
    fn closed_outbound_stream_releases_runtime_capacity() {
        let stream_id = StreamId::from_bytes([0x52; 16]);
        let mut streams = RuntimeStreams::new(NonZeroUsize::new(1).unwrap());
        streams.outbound.push(OutboundStream {
            stream_id,
            stream: Box::new(ClosedSendStream),
        });
        let payload = b"unsent-private-bytes".to_vec();

        assert_eq!(
            streams.try_send_chunk(stream_id, payload.clone()),
            Err(StreamSendError::Closed(payload))
        );
        assert!(streams.outbound.is_empty());
    }
}
