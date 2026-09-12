use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalError {
    Setup,
    Connect,
    Control,
    Bulk,
    Timeout,
}

impl fmt::Display for EvalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Setup => "failed to set up networking evaluation fixture",
            Self::Connect => "failed to establish protected transport connection",
            Self::Control => "control round trip failed",
            Self::Bulk => "bulk transfer failed",
            Self::Timeout => "networking evaluation timed out",
        })
    }
}

impl std::error::Error for EvalError {}
