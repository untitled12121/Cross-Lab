use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileRuntimeError {
    AlreadyStarted,
    NotStarted,
    StateUnavailable,
}

impl fmt::Display for MobileRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AlreadyStarted => "mobile runtime is already started",
            Self::NotStarted => "mobile runtime is not started",
            Self::StateUnavailable => "mobile runtime state is unavailable",
        })
    }
}

impl std::error::Error for MobileRuntimeError {}
