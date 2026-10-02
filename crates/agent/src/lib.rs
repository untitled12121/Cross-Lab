//! Shared unprivileged Cross-Lab agent coordination.

mod clipboard;
mod file_transfer;
mod notification;
mod presence;

pub use clipboard::{
    CLIPBOARD_TEXT_MAX_BYTES, ClipboardAvailability, ClipboardOperationError,
    ClipboardPlatformError, ClipboardRequest,
};
pub use file_transfer::{
    FileTransferAvailability, FileTransferCancellation, FileTransferChunkError,
    FileTransferCompletionTombstone, FileTransferDataChunk, FileTransferDataEvent,
    FileTransferHash, FileTransferHasher, FileTransferIdentity, FileTransferIntegrityError,
    FileTransferLocalLocator, FileTransferOperationError, FileTransferPartialState,
    FileTransferRecoveryAction, FileTransferRequest, FileTransferRetainedState,
    FileTransferSourceStream, FileTransferStateError, FileTransferStateMatch,
    FileTransferStateSnapshot, FileTransferVerifier, MAX_FILE_TRANSFER_LOCAL_LOCATOR_BYTES,
    MAX_FILE_TRANSFER_STATE_BYTES, MAX_RETAINED_FILE_TRANSFERS,
};
pub use notification::{
    NOTIFICATION_QUEUE_CAPACITY, NotificationConsent, NotificationInbox,
    NotificationInboxSnapshot, NotificationInboxStatus, NotificationMirror, NotificationMirrorError,
    NotificationRole, PlatformNotification,
};
pub use presence::{
    PermissionRule, PermissionSnapshot, PresenceAgentError, PresenceDiscoveryInfo, PresencePhase,
    PresenceSnapshot, TrustedPresenceAgent, TrustedSessionRoute,
};
