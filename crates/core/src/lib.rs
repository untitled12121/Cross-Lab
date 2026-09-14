//! Shared local-first runtime core for Cross-Lab.

mod control;
mod pairing;
mod session;
mod stream;
mod transport;

pub use control::{ControlDispatchError, ControlDispatcher, EventSubscription, InboundControl};
pub use pairing::{
    INITIAL_CREDENTIAL_EPOCH, InvitationState, PairingConfirmationError, PairingConfirmationRole,
    PairingFlowError, PairingId, PairingInstant, PairingInvitation, PairingInvitationError,
    PairingInviterFlow, PairingInviterState, PairingJoinerFlow, PairingJoinerState, PairingSecret,
    PairingTranscript,
};
pub use session::{
    LogicalSession, NegotiatedCapability, SessionActivation, SessionAuthError, SessionAuthProof,
    SessionAuthRole, SessionAuthTranscriptV1, SessionError, SessionHandshakeSide, SessionState,
};
pub use stream::{StreamAdmission, StreamAdmissionError};
pub use transport::{
    ChannelBinding, ConnectionMetadata, ControlReceiveError, ControlSendError, IncomingUniStream,
    MemoryTransportConnection, MemoryTransportPair, StreamAcceptError, StreamOpenError,
    StreamReceiveError, StreamSendError, TransportConnection, TransportReceiveStream,
    TransportSecurityClass, TransportSendStream,
};
