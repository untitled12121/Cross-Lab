use std::{num::NonZeroUsize, sync::Arc};

use crosslab_core::{
    ControlDispatchError, ControlDispatcher, ControlReceiveError, ControlSendError,
    EventSubscription, InboundControl, LogicalSession, SessionError, SessionState,
    StreamAcceptError, StreamOpenError, StreamSendError, TransportConnection,
};
use crosslab_identity::OwnerAuthorityState;
use crosslab_policy::{
    ApprovalInstant, AuthorizedOperation, DecisionReason, LocalCapability, NetworkClass,
    PolicyState, TrustRecord,
};
use crosslab_protocol::{
    CancelRequest, CapabilityAdvertisement, ControlRequest, ControlResponse, ControlResponseResult,
    DataStreamOpen, EnvelopeBody, Event, ProtocolFailure, ProtocolWireError, RequestId,
    SessionClose, SessionCloseReason, StreamId, decode_control_envelope, encode_control_envelope,
};

use crate::{
    RuntimeStatus,
    stream::{RuntimeStreamError, RuntimeStreamEvent, RuntimeStreams},
};

#[derive(Debug)]
pub enum NodeError {
    Session(SessionError),
    Wire(ProtocolWireError),
    Dispatch(ControlDispatchError),
    Send(ControlSendError),
    Receive(ControlReceiveError),
    Stream(RuntimeStreamError),
    PolicyRevisionRollback,
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
    Stream(RuntimeStreamEvent),
}

enum RuntimeTransport<'a> {
    Borrowed(&'a (dyn TransportConnection + Send + Sync)),
    Owned(Arc<dyn TransportConnection + Send + Sync>),
}

impl RuntimeTransport<'_> {
    fn as_ref(&self) -> &(dyn TransportConnection + Send + Sync) {
        match self {
            Self::Borrowed(transport) => *transport,
            Self::Owned(transport) => transport.as_ref(),
        }
    }
}

pub struct RuntimeNode<'a> {
    session: LogicalSession,
    dispatcher: ControlDispatcher,
    streams: RuntimeStreams,
    transport: RuntimeTransport<'a>,
    policy: PolicyState,
    local_capabilities: Vec<LocalCapability>,
    network_class: NetworkClass,
}

impl<'a> RuntimeNode<'a> {
    pub fn new(
        session: LogicalSession,
        transport: &'a (dyn TransportConnection + Send + Sync),
        policy: PolicyState,
        local_capabilities: Vec<LocalCapability>,
        network_class: NetworkClass,
        state_capacity: NonZeroUsize,
    ) -> Result<Self, NodeError> {
        build_runtime(
            session,
            RuntimeTransport::Borrowed(transport),
            policy,
            local_capabilities,
            network_class,
            state_capacity,
        )
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

    pub fn status(&self, peer_trust: &TrustRecord) -> RuntimeStatus {
        RuntimeStatus::from_runtime(
            &self.session,
            self.transport.as_ref(),
            peer_trust,
            self.network_class,
        )
    }

    pub const fn policy_revision(&self) -> u64 {
        self.policy.revision()
    }

    pub fn replace_policy(&mut self, policy: PolicyState) -> Result<bool, NodeError> {
        if self.policy == policy {
            return Ok(false);
        }
        if policy.revision() <= self.policy.revision() {
            return Err(NodeError::PolicyRevisionRollback);
        }
        self.dispatcher.cancel_session_state();
        self.streams.cancel_all();
        self.policy = policy;
        Ok(true)
    }

    pub fn subscribe_event(&mut self, subscription: EventSubscription) -> Result<bool, NodeError> {
        if self.session.state() != SessionState::Active {
            return Err(NodeError::Session(SessionError::InvalidState));
        }
        self.dispatcher
            .subscribe_event(subscription)
            .map_err(NodeError::Dispatch)
    }

    pub fn unsubscribe_event(
        &mut self,
        subscription: &EventSubscription,
    ) -> Result<bool, NodeError> {
        if self.session.state() != SessionState::Active {
            return Err(NodeError::Session(SessionError::InvalidState));
        }
        Ok(self.dispatcher.unsubscribe_event(subscription))
    }

    pub fn register_stream_operation(
        &mut self,
        operation: AuthorizedOperation,
    ) -> Result<(), NodeError> {
        self.streams
            .register_operation(&self.session, operation)
            .map_err(NodeError::Stream)
    }

    pub fn open_data_stream(&mut self, open: &DataStreamOpen) -> Result<StreamId, NodeError> {
        match self
            .streams
            .open_uni(&self.session, self.transport.as_ref(), open)
        {
            Ok(stream_id) => Ok(stream_id),
            Err(error @ RuntimeStreamError::Open(StreamOpenError::Closed(_))) => {
                self.terminate_transport_loss();
                Err(NodeError::Stream(error))
            }
            Err(error) => Err(NodeError::Stream(error)),
        }
    }

    pub fn try_send_stream_chunk(
        &mut self,
        stream_id: StreamId,
        chunk: Vec<u8>,
    ) -> Result<(), StreamSendError> {
        let result = self.streams.try_send_chunk(stream_id, chunk);
        if matches!(&result, Err(StreamSendError::Closed(_)))
            && self.transport.as_ref().is_closed()
        {
            self.terminate_transport_loss();
        }
        result
    }

    pub fn finish_data_stream(&mut self, stream_id: StreamId) -> Result<(), NodeError> {
        self.streams
            .finish_outbound(stream_id)
            .map_err(NodeError::Stream)
    }

    pub fn cancel_outbound_stream(&mut self, stream_id: StreamId) -> Result<(), NodeError> {
        self.streams
            .cancel_outbound(stream_id)
            .map_err(NodeError::Stream)
    }

    pub fn cancel_inbound_stream(&mut self, stream_id: StreamId) -> Result<(), NodeError> {
        self.streams
            .cancel_inbound(stream_id)
            .map_err(NodeError::Stream)
    }

    pub fn receive_stream_one(
        &mut self,
        peer_trust: &TrustRecord,
        now: u64,
    ) -> Result<NodeEvent, NodeError> {
        match self.streams.receive_one(
            &self.session,
            self.transport.as_ref(),
            now,
            peer_trust,
            &self.policy,
        ) {
            Ok(event) => {
                if matches!(&event, RuntimeStreamEvent::Cancelled(_))
                    && self.transport.as_ref().is_closed()
                {
                    self.terminate_transport_loss();
                }
                Ok(NodeEvent::Stream(event))
            }
            Err(error @ RuntimeStreamError::Accept(StreamAcceptError::Closed)) => {
                self.terminate_transport_loss();
                Err(NodeError::Stream(error))
            }
            Err(error) => Err(NodeError::Stream(error)),
        }
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
        self.streams.cancel_all();
        self.session.begin_close().map_err(NodeError::Session)
    }

    pub fn apply_peer_revocation(&mut self, peer_trust: &TrustRecord) -> Result<(), NodeError> {
        self.session
            .apply_peer_revocation(peer_trust)
            .map_err(NodeError::Session)?;
        self.dispatcher.cancel_session_state();
        self.streams.cancel_all();
        self.transport.as_ref().close();
        self.session.finish_close().map_err(NodeError::Session)
    }

    pub fn revalidate_authority(
        &mut self,
        authority: &OwnerAuthorityState,
    ) -> Result<(), NodeError> {
        if let Err(error) = self.session.revalidate_authority(authority) {
            self.dispatcher.cancel_session_state();
            self.streams.cancel_all();
            self.transport.as_ref().close();
            return Err(NodeError::Session(error));
        }
        Ok(())
    }

    pub fn network_lost(&mut self) {
        self.dispatcher.cancel_session_state();
        self.streams.cancel_all();
        let _ = self.session.transport_lost();
        self.transport.as_ref().close();
    }

    pub fn shutdown(&mut self) {
        self.dispatcher.cancel_session_state();
        self.streams.cancel_all();
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
        self.transport.as_ref().close();
    }

    pub fn receive_one(&mut self, peer_trust: &TrustRecord) -> Result<NodeEvent, NodeError> {
        self.receive_one_at(peer_trust, ApprovalInstant::from_ticks(0))
    }

    pub fn receive_one_at(
        &mut self,
        peer_trust: &TrustRecord,
        local_time: ApprovalInstant,
    ) -> Result<NodeEvent, NodeError> {
        let frame = match self.transport.as_ref().try_receive_control() {
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
                peer_trust,
                self.network_class,
                local_time,
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
        if let Err(error) = self.transport.as_ref().try_send_control(frame) {
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
        self.streams.cancel_all();
        let _ = self.session.transport_lost();
    }

    fn close_received(&mut self) -> Result<(), NodeError> {
        self.dispatcher.cancel_session_state();
        self.streams.cancel_all();
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
        self.transport.as_ref().close();
        Ok(())
    }

    fn fail_closed(&mut self) {
        self.dispatcher.cancel_session_state();
        self.streams.cancel_all();
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
        self.transport.as_ref().close();
    }
}

impl RuntimeNode<'static> {
    pub fn new_owned<T>(
        session: LogicalSession,
        transport: Arc<T>,
        policy: PolicyState,
        local_capabilities: Vec<LocalCapability>,
        network_class: NetworkClass,
        state_capacity: NonZeroUsize,
    ) -> Result<Self, NodeError>
    where
        T: TransportConnection + Send + Sync + 'static,
    {
        let transport: Arc<dyn TransportConnection + Send + Sync> = transport;
        build_runtime(
            session,
            RuntimeTransport::Owned(transport),
            policy,
            local_capabilities,
            network_class,
            state_capacity,
        )
    }
}

impl Drop for RuntimeNode<'_> {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn build_runtime<'a>(
    session: LogicalSession,
    transport: RuntimeTransport<'a>,
    policy: PolicyState,
    local_capabilities: Vec<LocalCapability>,
    network_class: NetworkClass,
    state_capacity: NonZeroUsize,
) -> Result<RuntimeNode<'a>, NodeError> {
    if session.state() != SessionState::Active {
        return Err(NodeError::Session(SessionError::InvalidState));
    }
    let context = session
        .context()
        .ok_or(NodeError::Session(SessionError::InvalidState))?;
    let dispatcher = ControlDispatcher::new(context, state_capacity);
    let streams = RuntimeStreams::new(state_capacity);
    Ok(RuntimeNode {
        session,
        dispatcher,
        streams,
        transport,
        policy,
        local_capabilities,
        network_class,
    })
}

const fn is_fatal_dispatch(error: ControlDispatchError) -> bool {
    matches!(
        error,
        ControlDispatchError::InvalidSession
            | ControlDispatchError::IncompatibleProtocol
            | ControlDispatchError::PeerTrustMismatch
            | ControlDispatchError::PeerCredentialEpochChanged
            | ControlDispatchError::PeerTrustRevisionChanged
            | ControlDispatchError::Sequence(_)
            | ControlDispatchError::ResourceLimit
            | ControlDispatchError::AuthorizationDenied(DecisionReason::UntrustedPeer)
            | ControlDispatchError::AuthorizationDenied(DecisionReason::RevokedPeer)
    )
}
