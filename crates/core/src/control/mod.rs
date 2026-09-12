use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
};

use crosslab_policy::{
    AuthorizationContext, DecisionEffect, DecisionReason, LocalCapability, NetworkClass,
    PolicyState, TrustState,
};
use crosslab_protocol::{
    ControlEnvelope, ControlRequest, ControlResponse, ControlSequence, EnvelopeBody, Event,
    EventScope, ProtocolFailure, RequestId, RetryClass, SequenceError, SessionClose,
};

use crate::SessionContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlDispatchError {
    InvalidSession,
    IncompatibleProtocol,
    Sequence(SequenceError),
    DuplicateRequest,
    UnknownRequest,
    ResourceLimit,
    CapabilityUnsupported,
    CapabilityVersionIncompatible,
    AuthorizationDenied(DecisionReason),
    ApprovalRequired,
    InvalidEventCapability,
}

impl From<SequenceError> for ControlDispatchError {
    fn from(error: SequenceError) -> Self {
        Self::Sequence(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundControl {
    CapabilityAdvertisement(crosslab_protocol::CapabilityAdvertisement),
    Request(ControlRequest),
    Response(ControlResponse),
    Event(Event),
    Cancelled(RequestId),
    ProtocolFailure(ProtocolFailure),
    SessionClose(SessionClose),
}

#[derive(Debug)]
pub struct ControlDispatcher {
    receive_sequence: ControlSequence,
    next_send_sequence: Option<u64>,
    pending_outgoing: BTreeMap<RequestId, RetryClass>,
    inbound_requests: BTreeMap<RequestId, RetryClass>,
    seen_nonretryable: BTreeSet<RequestId>,
    state_capacity: usize,
}

impl ControlDispatcher {
    pub fn new(context: &SessionContext, state_capacity: NonZeroUsize) -> Self {
        Self {
            receive_sequence: ControlSequence::from_expected(context.next_receive_sequence()),
            next_send_sequence: Some(context.next_send_sequence()),
            pending_outgoing: BTreeMap::new(),
            inbound_requests: BTreeMap::new(),
            seen_nonretryable: BTreeSet::new(),
            state_capacity: state_capacity.get(),
        }
    }

    pub const fn next_send_sequence(&self) -> Option<u64> {
        self.next_send_sequence
    }

    pub const fn expected_receive_sequence(&self) -> Option<u64> {
        self.receive_sequence.expected()
    }

    pub fn pending_request_count(&self) -> usize {
        self.pending_outgoing.len()
    }

    pub fn prepare_outbound(
        &self,
        context: &SessionContext,
        body: EnvelopeBody,
    ) -> Result<ControlEnvelope, ControlDispatchError> {
        self.validate_outbound_body(&body)?;
        let sequence = self
            .next_send_sequence
            .ok_or(ControlDispatchError::Sequence(SequenceError::Exhausted))?;
        Ok(ControlEnvelope::new(
            context.protocol_version(),
            context.session_id(),
            sequence,
            body,
        ))
    }

    pub fn commit_outbound(&mut self, envelope: &ControlEnvelope) {
        match envelope.body() {
            EnvelopeBody::ControlRequest(request) => {
                self.pending_outgoing
                    .insert(request.request_id(), request.retry_class());
            }
            EnvelopeBody::ControlResponse(response) => {
                self.inbound_requests.remove(&response.request_id());
            }
            EnvelopeBody::CancelRequest(cancel) => {
                self.pending_outgoing.remove(&cancel.request_id());
            }
            EnvelopeBody::CapabilityAdvertisement(_)
            | EnvelopeBody::Event(_)
            | EnvelopeBody::ProtocolError(_)
            | EnvelopeBody::SessionClose(_) => {}
        }
        self.next_send_sequence = envelope.message_seq().checked_add(1);
    }

    pub fn accept_inbound(
        &mut self,
        context: &SessionContext,
        envelope: ControlEnvelope,
        policy: &PolicyState,
        local_capabilities: &[LocalCapability],
        network_class: NetworkClass,
    ) -> Result<InboundControl, ControlDispatchError> {
        if envelope.session_id() != context.session_id() {
            return Err(ControlDispatchError::InvalidSession);
        }
        if envelope.protocol_version() != context.protocol_version() {
            return Err(ControlDispatchError::IncompatibleProtocol);
        }
        self.receive_sequence.accept(envelope.message_seq())?;

        match envelope.body() {
            EnvelopeBody::CapabilityAdvertisement(advertisement) => Ok(
                InboundControl::CapabilityAdvertisement(advertisement.clone()),
            ),
            EnvelopeBody::ControlRequest(request) => {
                self.accept_request(context, request, policy, local_capabilities, network_class)
            }
            EnvelopeBody::ControlResponse(response) => {
                if self
                    .pending_outgoing
                    .remove(&response.request_id())
                    .is_none()
                {
                    return Err(ControlDispatchError::UnknownRequest);
                }
                Ok(InboundControl::Response(response.clone()))
            }
            EnvelopeBody::Event(event) => {
                self.validate_event(context, event)?;
                Ok(InboundControl::Event(event.clone()))
            }
            EnvelopeBody::CancelRequest(cancel) => {
                if self.inbound_requests.remove(&cancel.request_id()).is_none() {
                    return Err(ControlDispatchError::UnknownRequest);
                }
                Ok(InboundControl::Cancelled(cancel.request_id()))
            }
            EnvelopeBody::ProtocolError(error) => {
                Ok(InboundControl::ProtocolFailure(error.clone()))
            }
            EnvelopeBody::SessionClose(close) => Ok(InboundControl::SessionClose(close.clone())),
        }
    }

    fn validate_outbound_body(&self, body: &EnvelopeBody) -> Result<(), ControlDispatchError> {
        match body {
            EnvelopeBody::ControlRequest(request) => {
                if self.pending_outgoing.contains_key(&request.request_id()) {
                    return Err(ControlDispatchError::DuplicateRequest);
                }
                if self.pending_outgoing.len() >= self.state_capacity {
                    return Err(ControlDispatchError::ResourceLimit);
                }
            }
            EnvelopeBody::ControlResponse(response) => {
                if !self.inbound_requests.contains_key(&response.request_id()) {
                    return Err(ControlDispatchError::UnknownRequest);
                }
            }
            EnvelopeBody::CancelRequest(cancel) => {
                if !self.pending_outgoing.contains_key(&cancel.request_id()) {
                    return Err(ControlDispatchError::UnknownRequest);
                }
            }
            EnvelopeBody::CapabilityAdvertisement(_)
            | EnvelopeBody::Event(_)
            | EnvelopeBody::ProtocolError(_)
            | EnvelopeBody::SessionClose(_) => {}
        }
        Ok(())
    }

    fn accept_request(
        &mut self,
        context: &SessionContext,
        request: &ControlRequest,
        policy: &PolicyState,
        local_capabilities: &[LocalCapability],
        network_class: NetworkClass,
    ) -> Result<InboundControl, ControlDispatchError> {
        let negotiated = context
            .negotiated_capabilities()
            .iter()
            .find(|capability| capability.capability_id() == request.capability_id())
            .ok_or(ControlDispatchError::CapabilityUnsupported)?;
        if negotiated.version() != request.capability_version() {
            return Err(ControlDispatchError::CapabilityVersionIncompatible);
        }

        if self.inbound_requests.contains_key(&request.request_id())
            || (request.retry_class() == RetryClass::NonRetryable
                && self.seen_nonretryable.contains(&request.request_id()))
        {
            return Err(ControlDispatchError::DuplicateRequest);
        }

        let local = local_capabilities
            .iter()
            .find(|capability| capability.capability_id() == request.capability_id())
            .ok_or(ControlDispatchError::CapabilityUnsupported)?;
        if !local.runtime_available() {
            return Err(ControlDispatchError::CapabilityUnsupported);
        }
        if !local.supports(request.capability_version()) {
            return Err(ControlDispatchError::CapabilityVersionIncompatible);
        }

        let authorization = AuthorizationContext::new(
            context.peer_device_id(),
            context.local_device_id(),
            context.session_id(),
            request.capability_id().clone(),
            request.capability_version(),
            request.operation_name().clone(),
            TrustState::Trusted,
            context.peer_trust_revision(),
            local.clone(),
            network_class,
        );
        let decision = policy.evaluate(&authorization);
        match decision.effect() {
            DecisionEffect::Deny => {
                return Err(ControlDispatchError::AuthorizationDenied(decision.reason()));
            }
            DecisionEffect::Ask => return Err(ControlDispatchError::ApprovalRequired),
            DecisionEffect::Allow => {}
        }

        if self.inbound_requests.len() >= self.state_capacity {
            return Err(ControlDispatchError::ResourceLimit);
        }
        if request.retry_class() == RetryClass::NonRetryable
            && self.seen_nonretryable.len() >= self.state_capacity
        {
            return Err(ControlDispatchError::ResourceLimit);
        }

        self.inbound_requests
            .insert(request.request_id(), request.retry_class());
        if request.retry_class() == RetryClass::NonRetryable {
            self.seen_nonretryable.insert(request.request_id());
        }
        Ok(InboundControl::Request(request.clone()))
    }

    fn validate_event(
        &self,
        context: &SessionContext,
        event: &Event,
    ) -> Result<(), ControlDispatchError> {
        let EventScope::Capability(capability_id) = event.scope() else {
            return Ok(());
        };
        if context
            .negotiated_capabilities()
            .iter()
            .any(|capability| capability.capability_id() == capability_id)
        {
            Ok(())
        } else {
            Err(ControlDispatchError::InvalidEventCapability)
        }
    }
}
