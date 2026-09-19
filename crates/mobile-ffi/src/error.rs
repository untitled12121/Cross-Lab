use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileRuntimeError {
    AlreadyStarted,
    NotStarted,
    StateUnavailable,
    DevelopmentUnavailable,
    DevelopmentAlreadyConfigured,
    DevelopmentProvisioning,
}

impl fmt::Display for MobileRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AlreadyStarted => "mobile runtime is already started",
            Self::NotStarted => "mobile runtime is not started",
            Self::StateUnavailable => "mobile runtime state is unavailable",
            Self::DevelopmentUnavailable => "development provisioning is unavailable",
            Self::DevelopmentAlreadyConfigured => "development provisioning is already configured",
            Self::DevelopmentProvisioning => "development provisioning is invalid",
        })
    }
}

impl std::error::Error for MobileRuntimeError {}
