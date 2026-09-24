//! Shared unprivileged Cross-Lab agent coordination.

mod presence;

pub use presence::{
    PermissionRule, PermissionSnapshot, PresenceAgentError, PresenceDiscoveryInfo, PresencePhase,
    PresenceSnapshot, TrustedPresenceAgent, TrustedSessionRoute,
};
