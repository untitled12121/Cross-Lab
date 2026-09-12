//! Platform-independent Cross-Lab coordination core.

mod control;
mod pairing;
mod session;
mod stream;
mod transport;

pub use control::{ControlDispatchError, ControlDispatcher, InboundControl};
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
pub use stream::{AdmittedStream, StreamAdmission, StreamAdmissionError};
pub use transport::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, TransportConnection,
    TransportSecurityClass,
};
