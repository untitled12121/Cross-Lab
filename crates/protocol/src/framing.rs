use core::fmt;

const LENGTH_PREFIX_BYTES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameLimit {
    BootstrapHello,
    NormalControl,
    DataStreamOpen,
}

impl FrameLimit {
    pub const fn max_payload_len(self) -> usize {
        match self {
            Self::BootstrapHello => 65_536,
            Self::NormalControl => 262_144,
            Self::DataStreamOpen => 4_096,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    MissingLengthPrefix,
    FrameTooLarge { declared: usize, max: usize },
    Truncated { declared: usize, available: usize },
    TrailingBytes { declared: usize, available: usize },
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLengthPrefix => formatter.write_str("frame is missing its length prefix"),
            Self::FrameTooLarge { declared, max } => {
                write!(formatter, "frame length {declared} exceeds maximum {max}")
            }
            Self::Truncated {
                declared,
                available,
            } => write!(
                formatter,
                "frame declares {declared} payload bytes but only {available} are available"
            ),
            Self::TrailingBytes {
                declared,
                available,
            } => write!(
                formatter,
                "frame declares {declared} payload bytes but contains {available}"
            ),
        }
    }
}

impl std::error::Error for FrameError {}

pub fn encode_frame(payload: &[u8], limit: FrameLimit) -> Result<Vec<u8>, FrameError> {
    let max = limit.max_payload_len();
    if payload.len() > max {
        return Err(FrameError::FrameTooLarge {
            declared: payload.len(),
            max,
        });
    }

    let mut frame = Vec::with_capacity(LENGTH_PREFIX_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

pub fn decode_frame(frame: &[u8], limit: FrameLimit) -> Result<&[u8], FrameError> {
    let prefix = frame
        .get(..LENGTH_PREFIX_BYTES)
        .ok_or(FrameError::MissingLengthPrefix)?;
    let declared = u32::from_be_bytes(prefix.try_into().expect("length prefix is four bytes"))
        as usize;
    let max = limit.max_payload_len();

    if declared > max {
        return Err(FrameError::FrameTooLarge { declared, max });
    }

    let available = frame.len() - LENGTH_PREFIX_BYTES;
    if available < declared {
        return Err(FrameError::Truncated {
            declared,
            available,
        });
    }
    if available > declared {
        return Err(FrameError::TrailingBytes {
            declared,
            available,
        });
    }

    Ok(&frame[LENGTH_PREFIX_BYTES..])
}
