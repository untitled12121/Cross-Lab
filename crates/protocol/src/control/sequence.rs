use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlSequence {
    expected: Option<u64>,
}

impl ControlSequence {
    pub const fn new() -> Self {
        Self { expected: Some(0) }
    }

    pub const fn from_expected(expected: u64) -> Self {
        Self {
            expected: Some(expected),
        }
    }

    pub const fn expected(self) -> Option<u64> {
        self.expected
    }

    pub fn accept(&mut self, received: u64) -> Result<(), SequenceError> {
        let expected = self.expected.ok_or(SequenceError::Exhausted)?;

        if received < expected {
            return Err(SequenceError::ReplayDetected { expected, received });
        }
        if received > expected {
            return Err(SequenceError::Gap { expected, received });
        }

        self.expected = expected.checked_add(1);
        Ok(())
    }
}

impl Default for ControlSequence {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceError {
    ReplayDetected { expected: u64, received: u64 },
    Gap { expected: u64, received: u64 },
    Exhausted,
}

impl fmt::Display for SequenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReplayDetected { expected, received } => write!(
                formatter,
                "control sequence replay: expected {expected}, received {received}"
            ),
            Self::Gap { expected, received } => write!(
                formatter,
                "control sequence gap: expected {expected}, received {received}"
            ),
            Self::Exhausted => formatter.write_str("control sequence space is exhausted"),
        }
    }
}

impl std::error::Error for SequenceError {}
