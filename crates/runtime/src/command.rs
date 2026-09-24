use crosslab_policy::{PolicyState, TrustRecord};
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
    Reconnect {
        session: Box<RuntimeActorSession>,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    Stop {
        reply: oneshot::Sender<()>,
    },
}
