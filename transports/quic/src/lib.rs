use core::fmt;

#[cfg_attr(not(test), allow(dead_code))]
mod binding;

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
mod tests;
