use core::fmt;

pub const MAX_DIAGNOSTIC_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolDiagnostic(String);

impl ProtocolDiagnostic {
    pub fn new(value: &str) -> Result<Self, DiagnosticError> {
        if value.len() > MAX_DIAGNOSTIC_BYTES {
            return Err(DiagnosticError::TooLong);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticError {
    TooLong,
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("protocol diagnostic exceeds 512 UTF-8 bytes")
    }
}

impl std::error::Error for DiagnosticError {}
