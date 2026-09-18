use core::fmt;

#[cfg_attr(not(test), allow(dead_code))]
mod binding;
mod config;
#[cfg(feature = "development-provisioning")]
pub mod development;
#[cfg_attr(not(test), allow(dead_code))]
mod connection;
mod endpoint;
#[cfg_attr(not(test), allow(dead_code))]
mod record;
mod session;
mod stream;

pub use config::QuicTransportConfig;
pub use connection::QuicTransportConnection;
pub use endpoint::{
    QuicClientEndpoint, QuicClientTlsConfig, QuicEndpointError, QuicServerEndpoint,
    QuicServerTlsConfig,
};
pub use session::{
    AuthenticatedQuicSession, QuicSessionAuthConfig, QuicSessionError, QuicSessionTimeouts,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicTransportError {
    ChannelBinding,
}

impl fmt::Display for QuicTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ChannelBinding => "failed to derive QUIC TLS exporter channel binding",
        })
    }
}

impl std::error::Error for QuicTransportError {}

#[cfg(test)]
mod session_auth_tests;
#[cfg(test)]
mod tests;
