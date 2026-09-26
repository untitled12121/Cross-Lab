//! Shared unprivileged Cross-Lab agent coordination.

mod clipboard;
mod presence;

pub use clipboard::{
    CLIPBOARD_TEXT_MAX_BYTES, ClipboardAvailability, ClipboardOperationError, ClipboardPlatformError,
    ClipboardRequest,
};
pub use presence::{
    PermissionRule, PermissionSnapshot, PresenceAgentError, PresenceDiscoveryInfo, PresencePhase,
    PresenceSnapshot, TrustedPresenceAgent, TrustedSessionRoute,
};
