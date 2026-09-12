use core::fmt;
use std::num::NonZeroUsize;

use crosslab_core::{
    LogicalSession, SessionError, SessionState, StreamAcceptError, StreamAdmission,
    StreamAdmissionError, StreamOpenError, StreamReceiveError, TransportConnection,
    TransportReceiveStream, TransportSendStream,
};
use crosslab_policy::{AuthorizedOperation, TrustRecord};
use crosslab_protocol::{
    DataStreamOpen, ProtocolWireError, StreamId, decode_data_stream_open, encode_data_stream_open,
};

#[derive(Debug)]
pub enum SimStreamError {
    Session(SessionError),
    Wire(ProtocolWireError),
    Admission(StreamAdmissionError),
    Open(StreamOpenError),
    Accept(StreamAcceptError),
    Receive(StreamReceiveError),
    StreamNotFound,
    ResourceLimit,
}

impl fmt::Display for SimStreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(error) => fmt::Display::fmt(error, formatter),
            Self::Wire(error) => fmt::Display::fmt(error, formatter),
            Self::Admission(error) => fmt::Display::fmt(error, formatter),
            Self::Open(error) => fmt::Display::fmt(error, formatter),
            Self::Accept(error) => fmt::Display::fmt(error, formatter),
            Self::Receive(error) => fmt::Display::fmt(error, formatter),
            Self::StreamNotFound => formatter.write_str("simulator data stream was not found"),
            Self::ResourceLimit => {
                formatter.write_str("simulator data stream capacity is exhausted")
            }
        }
    }
}

impl std::error::Error for SimStreamError {}

impl From<SessionError> for SimStreamError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

impl From<ProtocolWireError> for SimStreamError {
    fn from(error: ProtocolWireError) -> Self {
        Self::Wire(error)
    }
}

impl From<StreamAdmissionError> for SimStreamError {
    fn from(error: StreamAdmissionError) -> Self {
        Self::Admission(error)
    }
}

impl From<StreamOpenError> for SimStreamError {
    fn from(error: StreamOpenError) -> Self {
        Self::Open(error)
    }
}

impl From<StreamAcceptError> for SimStreamError {
    fn from(error: StreamAcceptError) -> Self {
        Self::Accept(error)
    }
}

impl From<StreamReceiveError> for SimStreamError {
    fn from(error: StreamReceiveError) -> Self {
        Self::Receive(error)
    }
}

struct InboundStream {
    stream_id: StreamId,
    stream: Box<dyn TransportReceiveStream>,
}

pub struct SimStreamRuntime<'a> {
    session: LogicalSession,
    transport: &'a dyn TransportConnection,
    admission: StreamAdmission,
    capacity: usize,
    inbound: Vec<InboundStream>,
}

impl<'a> SimStreamRuntime<'a> {
    pub fn new(
        session: LogicalSession,
        transport: &'a dyn TransportConnection,
        capacity: NonZeroUsize,
    ) -> Result<Self, SimStreamError> {
        ensure_active(&session)?;
        Ok(Self {
            session,
            transport,
            admission: StreamAdmission::new(capacity),
            capacity: capacity.get(),
            inbound: Vec::with_capacity(capacity.get()),
        })
    }

    pub const fn session(&self) -> &LogicalSession {
        &self.session
    }

    pub fn register_operation(
        &mut self,
        operation: AuthorizedOperation,
    ) -> Result<(), SimStreamError> {
        ensure_active(&self.session)?;
        self.admission.register_operation(operation)?;
        Ok(())
    }

    pub fn open_uni(
        &mut self,
        open: &DataStreamOpen,
    ) -> Result<Box<dyn TransportSendStream>, SimStreamError> {
        ensure_active(&self.session)?;
        let frame = encode_data_stream_open(open)?;
        match self.transport.try_open_uni_stream(frame) {
            Ok(stream) => Ok(stream),
            Err(error @ StreamOpenError::Full(_)) => Err(error.into()),
            Err(error @ StreamOpenError::Closed(_)) => {
                self.transport_lost();
                Err(error.into())
            }
        }
    }

    pub fn accept_one(
        &mut self,
        now: u64,
        current_trust_revision: u64,
        current_policy_revision: u64,
    ) -> Result<StreamId, SimStreamError> {
        ensure_active(&self.session)?;
        if self.inbound.len() >= self.capacity {
            return Err(SimStreamError::ResourceLimit);
        }

        let incoming = match self.transport.try_accept_uni_stream() {
            Ok(incoming) => incoming,
            Err(StreamAcceptError::Empty) => {
                return Err(SimStreamError::Accept(StreamAcceptError::Empty));
            }
            Err(StreamAcceptError::Closed) => {
                self.transport_lost();
                return Err(SimStreamError::Accept(StreamAcceptError::Closed));
            }
        };
        let (frame, mut stream) = incoming.into_parts();
        let open = match decode_data_stream_open(&frame) {
            Ok(open) => open,
            Err(error) => {
                stream.cancel();
                return Err(error.into());
            }
        };
        let admitted = match self.admission.admit_inbound(
            &self.session,
            &open,
            now,
            current_trust_revision,
            current_policy_revision,
        ) {
            Ok(admitted) => admitted,
            Err(error) => {
                stream.cancel();
                return Err(error.into());
            }
        };

        let stream_id = admitted.stream_id();
        self.inbound.push(InboundStream { stream_id, stream });
        Ok(stream_id)
    }

    pub fn try_receive_chunk(&mut self, stream_id: StreamId) -> Result<Vec<u8>, SimStreamError> {
        let position = self
            .inbound
            .iter()
            .position(|inbound| inbound.stream_id == stream_id)
            .ok_or(SimStreamError::StreamNotFound)?;
        let result = self.inbound[position].stream.try_receive_chunk();
        match result {
            Ok(chunk) => Ok(chunk),
            Err(StreamReceiveError::Empty) => {
                Err(SimStreamError::Receive(StreamReceiveError::Empty))
            }
            Err(StreamReceiveError::Finished) => {
                self.admission.finish_stream(stream_id)?;
                self.inbound.remove(position);
                Err(SimStreamError::Receive(StreamReceiveError::Finished))
            }
            Err(StreamReceiveError::Cancelled) if self.transport.is_closed() => {
                self.transport_lost();
                Err(SimStreamError::Receive(StreamReceiveError::Cancelled))
            }
            Err(StreamReceiveError::Cancelled) => {
                self.admission.cancel_stream(stream_id)?;
                self.inbound.remove(position);
                Err(SimStreamError::Receive(StreamReceiveError::Cancelled))
            }
        }
    }

    pub fn cancel_stream(&mut self, stream_id: StreamId) -> Result<(), SimStreamError> {
        let position = self
            .inbound
            .iter()
            .position(|inbound| inbound.stream_id == stream_id)
            .ok_or(SimStreamError::StreamNotFound)?;
        let mut inbound = self.inbound.remove(position);
        inbound.stream.cancel();
        self.admission.cancel_stream(stream_id)?;
        Ok(())
    }

    pub fn apply_peer_revocation(&mut self, peer_trust: &TrustRecord) -> Result<(), SimStreamError> {
        self.session.apply_peer_revocation(peer_trust)?;
        self.cancel_session_authority();
        self.transport.close();
        self.session.finish_close()?;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.cancel_session_authority();
        match self.session.state() {
            SessionState::Active => {
                let _ = self.session.begin_close();
                let _ = self.session.finish_close();
            }
            SessionState::Closing | SessionState::Revoked => {
                let _ = self.session.finish_close();
            }
            SessionState::Created | SessionState::Authenticating | SessionState::Closed => {}
        }
        self.transport.close();
    }

    fn transport_lost(&mut self) {
        self.cancel_session_authority();
        let _ = self.session.transport_lost();
    }

    fn cancel_session_authority(&mut self) {
        for inbound in &mut self.inbound {
            inbound.stream.cancel();
        }
        self.inbound.clear();
        self.admission.cancel_all();
    }
}

fn ensure_active(session: &LogicalSession) -> Result<(), SessionError> {
    if session.state() == SessionState::Active {
        Ok(())
    } else {
        Err(SessionError::InvalidState)
    }
}
