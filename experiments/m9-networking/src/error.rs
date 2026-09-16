use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalError {
    Setup,
    Connect,
    ChannelBinding,
    Control,
    Bulk,
    Timeout,
    InvalidCommand,
    InvalidArgument,
    InvalidValue,
}

impl fmt::Display for EvalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Setup => "failed to set up networking evaluation fixture",
            Self::Connect => "failed to establish protected transport connection",
            Self::ChannelBinding => "failed to derive protected transport channel binding",
            Self::Control => "control round trip failed",
            Self::Bulk => "bulk transfer failed",
            Self::Timeout => "networking evaluation timed out",
            Self::InvalidCommand => "unknown networking evaluation command",
            Self::InvalidArgument => "invalid networking evaluation argument",
            Self::InvalidValue => "invalid networking evaluation value",
        })
    }
}

impl std::error::Error for EvalError {}
