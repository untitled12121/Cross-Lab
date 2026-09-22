mod auth;
mod capabilities;
mod currentness;
mod discovery;
mod state;

pub use auth::{SessionAuthError, SessionAuthProof, SessionAuthRole, SessionAuthTranscriptV1};
pub use capabilities::NegotiatedCapability;
pub(crate) use capabilities::negotiate_session_capabilities;
pub use discovery::{
    MAX_SESSION_DISCOVERY_CANDIDATES, SESSION_DNS_SD_INSTANCE_NONCE_LEN,
    SESSION_DNS_SD_INSTANCE_PREFIX, SESSION_DNS_SD_SERVICE_TYPE, SESSION_DNS_SD_TXT_VERSION_KEY,
    SESSION_DNS_SD_TXT_VERSION_V1, is_session_dns_sd_instance, session_dns_sd_instance,
    should_initiate_session,
};
pub use state::{
    LogicalSession, SessionActivation, SessionContext, SessionError, SessionHandshakeSide,
    SessionState,
};
