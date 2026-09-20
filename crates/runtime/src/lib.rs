//! Shared unprivileged application runtime coordination.

mod actor;
mod command;
mod node;
mod pairing;
mod status;

pub use actor::{RuntimeActor, RuntimeActorConfig, RuntimeActorError, RuntimeActorSession};
pub use node::{NodeError, NodeEvent, RuntimeNode};
pub use pairing::{
    ProductPairingCommit, ProductPairingError, ProductPairingInviter, ProductPairingJoiner,
    ProductPairingState,
};
pub use status::{ConnectivityState, RuntimeStatus, TransportStatus};

#[cfg(test)]
mod tests;
