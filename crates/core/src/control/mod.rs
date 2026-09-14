use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    num::NonZeroUsize,
};

use crosslab_policy::{
    ApprovalInstant, AuthorizationContext, CapabilityId, DecisionEffect, DecisionReason,
    LocalCapability, NetworkClass, PolicyState, TrustRecord, TrustState,
};
use crosslab_protocol::{
    ControlEnvelope, ControlRequest, ControlResponse, ControlSequence, EnvelopeBody, Event,
    EventScope, EventType, ProtocolFailure, RequestId, RetryClass, SequenceError, SessionClose,
};

use crate::SessionContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlDispatchError {
    InvalidSession,
    IncompatibleProtocol,
    PeerTrustMismatch,
    PeerCredentialEpochChanged,
    PeerTrustRevisionChanged,
    Sequence(SequenceError),
    DuplicateRequest,
    UnknownRequest,
    ResourceLimit,
    CapabilityUnsupported,
    CapabilityVersionIncompatible,
    AuthorizationDenied(DecisionReason),
    ApprovalRequired,
    InvalidEventCapability,
    EventNotSubscribed,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventSubscription {
    capability_id: CapabilityId,
    event_type: EventType,
}

impl EventSubscription {
    pub fn new(capability_id: CapabilityId, event_type: EventType) -> Self {
        Self {
            capability_id,
            event_type,
        }
    }

    pub const fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
    }

    pub const fn event_type(&self) -> &EventType {
        &self.event_type
    }

    fn matches(&self, capability_id: &CapabilityId, event_type: &EventType) -> bool {
        self.capability_id == *capability_id && self.event_type == *event_type
    }
}

#[derive(Debug)]
pub struct ControlDispatcher {
    receive_sequence: ControlSequence,
    next_send_sequence: Option<u64>,
    pending_outgoing: BTreeMap<RequestId, RetryClass>,
    inbound_requests: BTreeSet<RequestId>,
    completed_inbound: BTreeSet<RequestId>,
    completed_order: VecDeque<RequestId>,
    event_subscriptions: BTreeSet<EventSubscription>,
    state_capacity: usize,
}

impl ControlDispatcher {
    pub fn new(context: &SessionContext, state_capacity: NonZeroUsize) -> Self {
        Self {
            receive_sequence: ControlSequence::from_expected(context.next_receive_sequence()),
            next_send_sequence: Some(context.next_send_sequence()),
            pending_outgoing: BTreeMap::new(),
            inbound_requests: BTreeSet::new(),
            completed_inbound: BTreeSet::new(),
            completed_order: VecDeque::new(),
            event_subscriptions: BTreeSet::new(),
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

    pub fn subscribe_event(&mut self, subscription: EventSubscription) -> bool {
        self.event_subscriptions.insert(subscription)
    }

    pub fn unsubscribe_event(&mut self, subscription: &EventSubscription) -> bool {
        self.event_subscriptions.remove(subscription)
    }

    pub fn cancel_session_state(&mut self) {
        self.pending_outgoing.clear();
        self.inbound_requests.clear();
        self.completed_inbound.clear();
        self.completed_order.clear();
        self.event_subscriptions.clear();
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
                if self.inbound_requests.remove(&response.request_id()) {
                    self.remember_completed_inbound(response.request_id());
                }
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

    #[allow(clippy::too_many_arguments)]
    pub fn accept_inbound(
        &mut self,
        context: &SessionContext,
        envelope: ControlEnvelope,
        policy: &PolicyState,
        local_capabilities: &[LocalCapability],
        peer_trust: &TrustRecord,
        network_class: NetworkClass,
        local_time: ApprovalInstant,
    ) -> Result<InboundControl, ControlDispatchError> {
        if envelope.session_id() != context.session_id() {
            return Err(ControlDispatchError::InvalidSession);
        }
        if envelope.protocol_version() != context.protocol_version() {
            return Err(ControlDispatchError::IncompatibleProtocol);
        }
        self.validate_peer_trust(context, peer_trust)?;
        self.receive_sequence.accept(envelope.message_seq())?;

        match envelope.body() {
            EnvelopeBody::CapabilityAdvertisement(advertisement) => Ok(
                InboundControl::CapabilityAdvertisement(advertisement.clone()),
            ),
            EnvelopeBody::ControlRequest(request) => self.accept_request(
                context,
                request,
                policy,
                local_capabilities,
                peer_trust,
                network_class,
                local_time,
            ),
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
                if !self.inbound_requests.remove(&cancel.request_id()) {
                    return Err(ControlDispatchError::UnknownRequest);
                }
                self.remember_completed_inbound(cancel.request_id());
                Ok(InboundControl::Cancelled(cancel.request_id()))
            }
            EnvelopeBody::ProtocolError(error) => {
                Ok(InboundControl::ProtocolFailure(error.clone()))
            }
            EnvelopeBody::SessionClose(close) => Ok(InboundControl::SessionClose(close.clone())),
        }
    }

    fn validate_peer_trust(
        &self,
        context: &SessionContext,
        peer_trust: &TrustRecord,
    ) -> Result<(), ControlDispatchError> {
        if peer_trust.owner_id() != context.owner_id()
            || peer_trust.device_id() != context.peer_device_id()
        {
            return Err(ControlDispatchError::PeerTrustMismatch);
        }
        match peer_trust.state() {
            TrustState::Pending => {
                return Err(ControlDispatchError::AuthorizationDenied(
                    DecisionReason::UntrustedPeer,
                ));
            }
            TrustState::Revoked => {
                return Err(ControlDispatchError::AuthorizationDenied(
                    DecisionReason::RevokedPeer,
                ));
            }
            TrustState::Trusted => {}
        }
        if peer_trust.accepted_credential_epoch() != context.peer_credential_epoch() {
            return Err(ControlDispatchError::PeerCredentialEpochChanged);
        }
        if peer_trust.trust_revision() != context.peer_trust_revision() {
            return Err(ControlDispatchError::PeerTrustRevisionChanged);
        }
        Ok(())
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
                if !self.inbound_requests.contains(&response.request_id()) {
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

    #[allow(clippy::too_many_arguments)]
    fn accept_request(
        &mut self,
        context: &SessionContext,
        request: &ControlRequest,
        policy: &PolicyState,
        local_capabilities: &[LocalCapability],
        peer_trust: &TrustRecord,
        network_class: NetworkClass,
        local_time: ApprovalInstant,
    ) -> Result<InboundControl, ControlDispatchError> {
        let negotiated = context
            .negotiated_capabilities()
            .iter()
            .find(|capability| capability.capability_id() == request.capability_id())
            .ok_or(ControlDispatchError::CapabilityUnsupported)?;
        if negotiated.version() != request.capability_version() {
            return Err(ControlDispatchError::CapabilityVersionIncompatible);
        }

        if self.inbound_requests.contains(&request.request_id())
            || self.completed_inbound.contains(&request.request_id())
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
            peer_trust.state(),
            peer_trust.trust_revision(),
            local.clone(),
            network_class,
        )
        .with_local_time(local_time);
        let decision = policy.evaluate(&authorization);
        match decision.effect() {
            DecisionEffect::Deny => {
                return Err(ControlDispatchError::AuthorizationDenied(decision.reason()));
            }
            DecisionEffect::Ask => return Err(ControlDispatchError::ApprovalRequired),
            DecisionEffect::Allow => {}
        }

        self.reserve_inbound_slot()?;
        self.inbound_requests.insert(request.request_id());
        Ok(InboundControl::Request(request.clone()))
    }

    fn reserve_inbound_slot(&mut self) -> Result<(), ControlDispatchError> {
        if self.inbound_requests.len() >= self.state_capacity {
            return Err(ControlDispatchError::ResourceLimit);
        }

        let completed_budget = self.state_capacity - self.inbound_requests.len() - 1;
        self.trim_completed_inbound(completed_budget);
        Ok(())
    }

    fn remember_completed_inbound(&mut self, request_id: RequestId) {
        if self.completed_inbound.insert(request_id) {
            self.completed_order.push_back(request_id);
        }
        let completed_budget = self
            .state_capacity
            .saturating_sub(self.inbound_requests.len());
        self.trim_completed_inbound(completed_budget);
    }

    fn trim_completed_inbound(&mut self, completed_budget: usize) {
        while self.completed_inbound.len() > completed_budget {
            let Some(request_id) = self.completed_order.pop_front() else {
                break;
            };
            self.completed_inbound.remove(&request_id);
        }
    }

    fn validate_event(
        &self,
        context: &SessionContext,
        event: &Event,
    ) -> Result<(), ControlDispatchError> {
        let EventScope::Capability(capability_id) = event.scope() else {
            return Ok(());
        };
        if !context
            .negotiated_capabilities()
            .iter()
            .any(|capability| capability.capability_id() == capability_id)
        {
            return Err(ControlDispatchError::InvalidEventCapability);
        }
        if !self
            .event_subscriptions
            .iter()
            .any(|subscription| subscription.matches(capability_id, event.event_type()))
        {
            return Err(ControlDispatchError::EventNotSubscribed);
        }
        Ok(())
    }
}
