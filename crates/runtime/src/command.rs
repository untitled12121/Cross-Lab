use tokio::sync::oneshot;

use crate::actor::{RuntimeActorError, RuntimeActorSession};

pub(crate) enum RuntimeCommand {
    NetworkLost,
    Reconnect {
        session: RuntimeActorSession,
        reply: oneshot::Sender<Result<(), RuntimeActorError>>,
    },
    Stop {
        reply: oneshot::Sender<()>,
    },
}
