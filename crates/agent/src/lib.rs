//! Shared unprivileged Cross-Lab agent coordination.

mod presence;

pub use presence::{
    PresenceAgentError, PresenceDiscoveryInfo, PresencePhase, PresenceSnapshot,
    TrustedPresenceAgent, TrustedSessionRoute,
};
