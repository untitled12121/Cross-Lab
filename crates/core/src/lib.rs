//! Platform-independent Cross-Lab coordination core.

mod control;
mod pairing;
mod session;
mod stream;
mod transport;

pub use control::{ControlDispatchError, ControlDispatcher, EventSubscription, InboundControl};
pub use pairing::{
    PAIRING_BOOTSTRAP_PROFILE_V1, PAIRING_DNS_SD_SERVICE_TYPE,
    PAIRING_DNS_SD_TXT_VERSION_KEY, PAIRING_DNS_SD_TXT_VERSION_V1, PairingBootstrap,
    PairingBootstrapCode, PairingBootstrapError, PairingConfirmationError, PairingConfirmationRole, PairingFlowError, PairingId, PairingInstant,
    PairingInvitation, PairingInvitationCreateError, PairingInvitationError,
    PairingInvitationState, PairingInviterFlow, PairingInviterState, PairingJoinerFlow,
    PairingJoinerState, PairingSecret, PairingTranscript, PairingTrustEstablishment,
    pairing_dns_sd_instance,
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
