use core::fmt;

use super::ProtocolDiagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCloseReason {
    Normal,
    LocalRequest,
    ProtocolError,
    AuthenticationLost,
    TrustRevoked,
    Shutdown,
}

impl SessionCloseReason {
    pub const fn code(self) -> u16 {
        match self {
            Self::Normal => 1,
            Self::LocalRequest => 2,
            Self::ProtocolError => 3,
            Self::AuthenticationLost => 4,
            Self::TrustRevoked => 5,
            Self::Shutdown => 6,
        }
    }

    pub fn from_code(value: u16) -> Result<Self, SessionCloseReasonError> {
        match value {
            1 => Ok(Self::Normal),
            2 => Ok(Self::LocalRequest),
            3 => Ok(Self::ProtocolError),
            4 => Ok(Self::AuthenticationLost),
            5 => Ok(Self::TrustRevoked),
            6 => Ok(Self::Shutdown),
            value => Err(SessionCloseReasonError::UnknownValue(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCloseReasonError {
    UnknownValue(u16),
}

impl fmt::Display for SessionCloseReasonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unknown session close reason")
    }
}

impl std::error::Error for SessionCloseReasonError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionClose {
    reason: SessionCloseReason,
    diagnostic: Option<ProtocolDiagnostic>,
}

impl SessionClose {
    pub const fn new(reason: SessionCloseReason, diagnostic: Option<ProtocolDiagnostic>) -> Self {
        Self { reason, diagnostic }
    }

    pub const fn reason(&self) -> SessionCloseReason {
        self.reason
    }

    pub const fn diagnostic(&self) -> Option<&ProtocolDiagnostic> {
        self.diagnostic.as_ref()
    }
}
