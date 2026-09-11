use core::fmt;

pub const MAX_PROTOCOL_RANGES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion {
    major: u16,
    minor: u16,
}

impl ProtocolVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub const fn major(self) -> u16 {
        self.major
    }

    pub const fn minor(self) -> u16 {
        self.minor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolRange {
    major: u16,
    min_minor: u16,
    max_minor: u16,
}

impl ProtocolRange {
    pub const fn new(
        major: u16,
        min_minor: u16,
        max_minor: u16,
    ) -> Result<Self, VersionNegotiationError> {
        if min_minor > max_minor {
            return Err(VersionNegotiationError::InvalidRange);
        }

        Ok(Self {
            major,
            min_minor,
            max_minor,
        })
    }

    pub const fn major(self) -> u16 {
        self.major
    }

    pub const fn min_minor(self) -> u16 {
        self.min_minor
    }

    pub const fn max_minor(self) -> u16 {
        self.max_minor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionNegotiationError {
    InvalidRange,
    TooManyRanges,
    IncompatibleProtocol,
}

impl fmt::Display for VersionNegotiationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRange => "protocol range has an invalid minor window",
            Self::TooManyRanges => "too many protocol ranges were advertised",
            Self::IncompatibleProtocol => "no compatible protocol version exists",
        })
    }
}

impl std::error::Error for VersionNegotiationError {}

pub fn negotiate_protocol_version(
    local: &[ProtocolRange],
    peer: &[ProtocolRange],
) -> Result<ProtocolVersion, VersionNegotiationError> {
    if local.len() > MAX_PROTOCOL_RANGES || peer.len() > MAX_PROTOCOL_RANGES {
        return Err(VersionNegotiationError::TooManyRanges);
    }

    let mut selected = None;

    for local_range in local {
        for peer_range in peer {
            if local_range.major != peer_range.major {
                continue;
            }

            let min_minor = local_range.min_minor.max(peer_range.min_minor);
            let max_minor = local_range.max_minor.min(peer_range.max_minor);
            if min_minor > max_minor {
                continue;
            }

            let candidate = ProtocolVersion::new(local_range.major, max_minor);
            if selected.is_none_or(|current| candidate > current) {
                selected = Some(candidate);
            }
        }
    }

    selected.ok_or(VersionNegotiationError::IncompatibleProtocol)
}
