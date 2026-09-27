use crosslab_core::EventSubscription;
use std::time::Duration;

use crosslab_policy::{
    CapabilityId, CapabilityVersion, OperationId, OperationName, PolicyState, TrustRecord,
    UsePolicy,
};
use crosslab_protocol::{
    CapabilityAdvertisement, ControlRequest, ControlResponseResult, RequestId,
};
use tokio::sync::oneshot;

use crate::actor::{RuntimeActorError, RuntimeActorSession};

pub(crate) enum RuntimeCommand {
    NetworkLost,
    PeerRevoked {
        peer_trust: TrustRecord,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    ReplacePolicy {
        policy: PolicyState,
        reply: oneshot::Sender<Result<bool, RuntimeActorError>>,
    },
    SendCapabilities {
        advertisement: CapabilityAdvertisement,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    SendRequest {
        request: ControlRequest,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    SendResponse {
        request_id: RequestId,
        result: ControlResponseResult,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    SendCancel {
        request_id: RequestId,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    SubscribeEvent {
        subscription: EventSubscription,
        reply: oneshot::Sender<Result<bool, RuntimeActorError>>,
    },
    UnsubscribeEvent {
        subscription: EventSubscription,
        reply: oneshot::Sender<Result<bool, RuntimeActorError>>,
    },
    IssueStreamOperation {
        capability_id: CapabilityId,
        capability_version: CapabilityVersion,
        operation: OperationName,
        lifetime: Duration,
        use_policy: UsePolicy,
        reply: oneshot::Sender<Result<OperationId, RuntimeActorError>>,
    },
    CancelStreamOperation {
        operation_id: OperationId,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    Reconnect {
        session: Box<RuntimeActorSession>,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    Stop {
        reply: oneshot::Sender<()>,
    },
}
