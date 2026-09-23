use crosslab_policy::{PolicyState, TrustRecord};
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
    Reconnect {
        session: Box<RuntimeActorSession>,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    Stop {
        reply: oneshot::Sender<()>,
    },
}
