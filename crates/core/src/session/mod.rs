mod auth;
mod capabilities;
mod state;

pub use auth::{SessionAuthError, SessionAuthProof, SessionAuthRole, SessionAuthTranscriptV1};
pub use capabilities::NegotiatedCapability;
pub(crate) use capabilities::negotiate_session_capabilities;
pub use state::{
    LogicalSession, SessionActivation, SessionContext, SessionError, SessionHandshakeSide,
    SessionState,
};
