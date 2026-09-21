//! Shared unprivileged application runtime coordination.

mod actor;
mod command;
mod node;
mod pairing;
mod status;

pub use actor::{RuntimeActor, RuntimeActorConfig, RuntimeActorError, RuntimeActorSession};
pub use node::{NodeError, NodeEvent, RuntimeNode};
pub use pairing::{
    ProductPairingCommit, ProductPairingError, ProductPairingExchangeState,
    ProductPairingInviter, ProductPairingInviterCompletion, ProductPairingInviterExchange,
    ProductPairingJoiner, ProductPairingJoinerCompletion, ProductPairingJoinerExchange,
    ProductPairingNetworkError, ProductPairingState, ProductPairingTrustBundle,
};
pub use status::{ConnectivityState, RuntimeStatus, TransportStatus};

#[cfg(test)]
mod tests;
