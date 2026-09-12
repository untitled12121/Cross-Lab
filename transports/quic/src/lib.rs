use core::fmt;

#[cfg_attr(not(test), allow(dead_code))]
mod binding;
mod config;
#[cfg_attr(not(test), allow(dead_code))]
mod connection;
#[cfg_attr(not(test), allow(dead_code))]
mod record;
mod stream;

pub use config::QuicTransportConfig;
pub use connection::QuicTransportConnection;

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
