//! Platform-independent Cross-Lab coordination core.

mod pairing;

pub use pairing::{
    PairingConfirmationError, PairingConfirmationRole, PairingId, PairingInvitation,
    PairingInvitationError, PairingInvitationState, PairingSecret, PairingTranscript,
};
