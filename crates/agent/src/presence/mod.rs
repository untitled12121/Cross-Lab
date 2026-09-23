mod agent;
mod runner;
mod types;

pub use agent::TrustedPresenceAgent;
pub use types::{
    PermissionRule, PermissionSnapshot, PresenceAgentError, PresenceDiscoveryInfo, PresencePhase,
    PresenceSnapshot, TrustedSessionRoute,
};

#[cfg(test)]
mod tests;
