//! Platform-independent Cross-Lab coordination core.

mod pairing;
mod session;
mod transport;

pub use pairing::{
    PairingConfirmationError, PairingConfirmationRole, PairingFlowError, PairingId,
    PairingInvitation, PairingInvitationError, PairingInvitationState, PairingInviterFlow,
    PairingInviterState, PairingJoinerFlow, PairingJoinerState, PairingSecret, PairingTranscript,
};
pub use session::{
    LogicalSession, NegotiatedCapability, SessionActivation, SessionAuthError, SessionAuthProof,
    SessionAuthRole, SessionAuthTranscriptV1, SessionContext, SessionError, SessionHandshakeSide,
    SessionState,
};
pub use transport::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, TransportConnection,
    TransportSecurityClass,
};
