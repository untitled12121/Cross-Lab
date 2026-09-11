use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    NonRetryable,
    Idempotent,
}

impl RetryClass {
    pub const fn code(self) -> u16 {
        match self {
            Self::NonRetryable => 0,
            Self::Idempotent => 1,
        }
    }

    pub const fn from_code(code: u16) -> Result<Self, RetryClassError> {
        match code {
            0 => Ok(Self::NonRetryable),
            1 => Ok(Self::Idempotent),
            value => Err(RetryClassError::UnknownValue(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClassError {
    UnknownValue(u16),
}

impl fmt::Display for RetryClassError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownValue(value) => write!(formatter, "unknown retry class value {value}"),
        }
    }
}

impl std::error::Error for RetryClassError {}
