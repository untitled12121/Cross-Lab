use core::fmt;

use super::ProtocolDiagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolErrorCode {
    MalformedFrame,
    FrameTooLarge,
    UnsupportedMessage,
    IncompatibleProtocol,
    UnsupportedRequiredFeature,
    InvalidIdentifier,
    InvalidSequence,
    ReplayDetected,
    DuplicateRequest,
    InvalidSession,
    AuthenticationFailed,
    TrustDenied,
    AuthorizationDenied,
    CapabilityUnsupported,
    CapabilityVersionIncompatible,
    OperationMissing,
    OperationExpired,
    OperationRevoked,
    OperationMismatch,
    Cancelled,
    ResourceLimit,
    InternalFailure,
}

impl ProtocolErrorCode {
    pub const fn code(self) -> u16 {
        match self {
            Self::MalformedFrame => 1,
            Self::FrameTooLarge => 2,
            Self::UnsupportedMessage => 3,
            Self::IncompatibleProtocol => 4,
            Self::UnsupportedRequiredFeature => 5,
            Self::InvalidIdentifier => 6,
            Self::InvalidSequence => 7,
            Self::ReplayDetected => 8,
            Self::DuplicateRequest => 9,
            Self::InvalidSession => 10,
            Self::AuthenticationFailed => 11,
            Self::TrustDenied => 12,
            Self::AuthorizationDenied => 13,
            Self::CapabilityUnsupported => 14,
            Self::CapabilityVersionIncompatible => 15,
            Self::OperationMissing => 16,
            Self::OperationExpired => 17,
            Self::OperationRevoked => 18,
            Self::OperationMismatch => 19,
            Self::Cancelled => 20,
            Self::ResourceLimit => 21,
            Self::InternalFailure => 22,
        }
    }

    pub const fn from_code(code: u16) -> Result<Self, ProtocolErrorCodeError> {
        match code {
            1 => Ok(Self::MalformedFrame),
            2 => Ok(Self::FrameTooLarge),
            3 => Ok(Self::UnsupportedMessage),
            4 => Ok(Self::IncompatibleProtocol),
            5 => Ok(Self::UnsupportedRequiredFeature),
            6 => Ok(Self::InvalidIdentifier),
            7 => Ok(Self::InvalidSequence),
            8 => Ok(Self::ReplayDetected),
            9 => Ok(Self::DuplicateRequest),
            10 => Ok(Self::InvalidSession),
            11 => Ok(Self::AuthenticationFailed),
            12 => Ok(Self::TrustDenied),
            13 => Ok(Self::AuthorizationDenied),
            14 => Ok(Self::CapabilityUnsupported),
            15 => Ok(Self::CapabilityVersionIncompatible),
            16 => Ok(Self::OperationMissing),
            17 => Ok(Self::OperationExpired),
            18 => Ok(Self::OperationRevoked),
            19 => Ok(Self::OperationMismatch),
            20 => Ok(Self::Cancelled),
            21 => Ok(Self::ResourceLimit),
            22 => Ok(Self::InternalFailure),
            value => Err(ProtocolErrorCodeError::UnknownValue(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolErrorCodeError {
    UnknownValue(u16),
}

impl fmt::Display for ProtocolErrorCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownValue(value) => write!(formatter, "unknown protocol error code {value}"),
        }
    }
}

impl std::error::Error for ProtocolErrorCodeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFailure {
    code: ProtocolErrorCode,
    diagnostic: Option<ProtocolDiagnostic>,
}

impl ProtocolFailure {
    pub fn new(code: ProtocolErrorCode, diagnostic: Option<ProtocolDiagnostic>) -> Self {
        Self { code, diagnostic }
    }

    pub const fn code(&self) -> ProtocolErrorCode {
        self.code
    }

    pub fn diagnostic(&self) -> Option<&ProtocolDiagnostic> {
        self.diagnostic.as_ref()
    }
}
