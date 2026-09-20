//! Platform-independent Cross-Lab coordination core.

mod control;
mod pairing;
mod session;
mod stream;
mod transport;

pub use control::{ControlDispatchError, ControlDispatcher, EventSubscription, InboundControl};
pub use pairing::{
    PAIRING_BOOTSTRAP_PROFILE_V1, PairingBootstrap, PairingBootstrapCode, PairingBootstrapError,
    PairingConfirmationError, PairingConfirmationRole, PairingFlowError, PairingId, PairingInstant,
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
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, IncomingUniStream,
    StreamAcceptError, StreamOpenError, StreamReceiveError, StreamSendError, TransportConnection,
    TransportReceiveStream, TransportSecurityClass, TransportSendStream,
};
