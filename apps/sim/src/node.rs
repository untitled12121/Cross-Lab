use std::num::NonZeroUsize;

use crosslab_core::{
    ControlDispatchError, ControlDispatcher, ControlReceiveError, ControlSendError, InboundControl,
    LogicalSession, SessionError, SessionState, TransportConnection,
};
use crosslab_policy::{LocalCapability, NetworkClass, PolicyState, TrustRecord};
use crosslab_protocol::{
    CancelRequest, CapabilityAdvertisement, ControlRequest, ControlResponse, ControlResponseResult,
    EnvelopeBody, Event, ProtocolFailure, ProtocolWireError, RequestId, SessionClose,
    SessionCloseReason, decode_control_envelope, encode_control_envelope,
};

#[derive(Debug)]
pub enum NodeError {
    Session(SessionError),
    Wire(ProtocolWireError),
    Dispatch(ControlDispatchError),
    Send(ControlSendError),
    Receive(ControlReceiveError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeEvent {
    CapabilitiesUpdated,
    RequestDispatched(ControlRequest),
    Response(ControlResponse),
    Event(Event),
    RequestCancelled(RequestId),
    ProtocolFailure(ProtocolFailure),
    SessionClosed(SessionCloseReason),
}

pub struct SimNode<'a> {
    session: LogicalSession,
    dispatcher: ControlDispatcher,
    transport: &'a dyn TransportConnection,
    policy: PolicyState,
    local_capabilities: Vec<LocalCapability>,
}

impl<'a> SimNode<'a> {
    pub fn new(
        session: LogicalSession,
        transport: &'a dyn TransportConnection,
        policy: PolicyState,
        local_capabilities: Vec<LocalCapability>,
        state_capacity: NonZeroUsize,
    ) -> Result<Self, NodeError> {
        if session.state() != SessionState::Active {
            return Err(NodeError::Session(SessionError::InvalidState));
        }
        let context = session
            .context()
            .ok_or(NodeError::Session(SessionError::InvalidState))?;
        let dispatcher = ControlDispatcher::new(context, state_capacity);
        Ok(Self {
            session,
            dispatcher,
            transport,
            policy,
            local_capabilities,
        })
    }

    pub const fn session(&self) -> &LogicalSession {
        &self.session
    }

    pub const fn next_send_sequence(&self) -> Option<u64> {
        self.dispatcher.next_send_sequence()
    }

    pub const fn expected_receive_sequence(&self) -> Option<u64> {
        self.dispatcher.expected_receive_sequence()
    }

    pub fn pending_request_count(&self) -> usize {
        self.dispatcher.pending_request_count()
    }

    pub fn send_request(&mut self, request: ControlRequest) -> Result<(), NodeError> {
        self.send_body(EnvelopeBody::ControlRequest(request))
    }

    pub fn send_response(
        &mut self,
        request_id: RequestId,
        result: ControlResponseResult,
    ) -> Result<(), NodeError> {
        self.send_body(EnvelopeBody::ControlResponse(ControlResponse::new(
            request_id, result,
        )))
    }

    pub fn send_event(&mut self, event: Event) -> Result<(), NodeError> {
        self.send_body(EnvelopeBody::Event(event))
    }

    pub fn send_cancel(&mut self, request_id: RequestId) -> Result<(), NodeError> {
        self.send_body(EnvelopeBody::CancelRequest(CancelRequest::new(request_id)))
    }

    pub fn send_capability_advertisement(
        &mut self,
        advertisement: CapabilityAdvertisement,
    ) -> Result<(), NodeError> {
        self.send_body(EnvelopeBody::CapabilityAdvertisement(advertisement))
    }

    pub fn send_close(&mut self, close: SessionClose) -> Result<(), NodeError> {
        self.send_body(EnvelopeBody::SessionClose(close))?;
        self.session.begin_close().map_err(NodeError::Session)
    }

    pub fn apply_peer_revocation(&mut self, peer_trust: &TrustRecord) -> Result<(), NodeError> {
        self.session
            .apply_peer_revocation(peer_trust)
            .map_err(NodeError::Session)?;
        self.dispatcher.cancel_session_state();
        self.transport.close();
        self.session.finish_close().map_err(NodeError::Session)
    }

    pub fn shutdown(&mut self) {
        self.dispatcher.cancel_session_state();
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

    pub fn receive_one(&mut self) -> Result<NodeEvent, NodeError> {
        let frame = match self.transport.try_receive_control() {
            Ok(frame) => frame,
            Err(error) => {
                if error == ControlReceiveError::Closed {
                    self.terminate_transport_loss();
                }
                return Err(NodeError::Receive(error));
            }
        };
        let envelope = match decode_control_envelope(&frame) {
            Ok(envelope) => envelope,
            Err(error) => {
                self.fail_closed();
                return Err(NodeError::Wire(error));
            }
        };

        let result = {
            let context = self
                .session
                .context()
                .ok_or(NodeError::Session(SessionError::InvalidState))?;
            self.dispatcher.accept_inbound(
                context,
                envelope,
                &self.policy,
                &self.local_capabilities,
                NetworkClass::Local,
            )
        };
        let inbound = match result {
            Ok(inbound) => inbound,
            Err(error) => {
                if is_fatal_dispatch(error) {
                    self.fail_closed();
                }
                return Err(NodeError::Dispatch(error));
            }
        };

        match inbound {
            InboundControl::CapabilityAdvertisement(advertisement) => {
                self.session
                    .negotiate_capabilities(&self.local_capabilities, &advertisement)
                    .map_err(NodeError::Session)?;
                Ok(NodeEvent::CapabilitiesUpdated)
            }
            InboundControl::Request(request) => Ok(NodeEvent::RequestDispatched(request)),
            InboundControl::Response(response) => Ok(NodeEvent::Response(response)),
            InboundControl::Event(event) => Ok(NodeEvent::Event(event)),
            InboundControl::Cancelled(request_id) => Ok(NodeEvent::RequestCancelled(request_id)),
            InboundControl::ProtocolFailure(error) => Ok(NodeEvent::ProtocolFailure(error)),
            InboundControl::SessionClose(close) => {
                let reason = close.reason();
                self.close_received()?;
                Ok(NodeEvent::SessionClosed(reason))
            }
        }
    }

    fn send_body(&mut self, body: EnvelopeBody) -> Result<(), NodeError> {
        if self.session.state() != SessionState::Active {
            return Err(NodeError::Session(SessionError::InvalidState));
        }
        let envelope = {
            let context = self
                .session
                .context()
                .ok_or(NodeError::Session(SessionError::InvalidState))?;
            self.dispatcher
                .prepare_outbound(context, body)
                .map_err(NodeError::Dispatch)?
        };
        let frame = encode_control_envelope(&envelope).map_err(NodeError::Wire)?;
        if let Err(error) = self.transport.try_send_control(frame) {
            if matches!(&error, ControlSendError::Closed(_)) {
                self.terminate_transport_loss();
            }
            return Err(NodeError::Send(error));
        }
        self.dispatcher.commit_outbound(&envelope);
        Ok(())
    }

    fn terminate_transport_loss(&mut self) {
        self.dispatcher.cancel_session_state();
        let _ = self.session.transport_lost();
    }

    fn close_received(&mut self) -> Result<(), NodeError> {
        match self.session.state() {
            SessionState::Active => {
                self.session.begin_close().map_err(NodeError::Session)?;
                self.session.finish_close().map_err(NodeError::Session)?;
            }
            SessionState::Closing | SessionState::Revoked => {
                self.session.finish_close().map_err(NodeError::Session)?;
            }
            SessionState::Closed => {}
            SessionState::Created | SessionState::Authenticating => {
                return Err(NodeError::Session(SessionError::InvalidState));
            }
        }
        self.transport.close();
        Ok(())
    }

    fn fail_closed(&mut self) {
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
}

const fn is_fatal_dispatch(error: ControlDispatchError) -> bool {
    matches!(
        error,
        ControlDispatchError::InvalidSession
            | ControlDispatchError::IncompatibleProtocol
            | ControlDispatchError::Sequence(_)
            | ControlDispatchError::ResourceLimit
    )
}
