//! Shared unprivileged Cross-Lab agent coordination.

mod clipboard;
mod file_transfer;
mod presence;

pub use clipboard::{
    CLIPBOARD_TEXT_MAX_BYTES, ClipboardAvailability, ClipboardOperationError,
    ClipboardPlatformError, ClipboardRequest,
};
pub use file_transfer::{
    FileTransferAvailability, FileTransferOperationError, FileTransferRequest,
};
pub use presence::{
    PermissionRule, PermissionSnapshot, PresenceAgentError, PresenceDiscoveryInfo, PresencePhase,
    PresenceSnapshot, TrustedPresenceAgent, TrustedSessionRoute,
};
