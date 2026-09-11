use core::fmt;

use crosslab_policy::{CapabilityId, CapabilityVersion};

pub const MAX_CAPABILITY_ADVERTISEMENT_ENTRIES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAdvertisementEntry {
    capability_id: CapabilityId,
    min_version: CapabilityVersion,
    max_version: CapabilityVersion,
    runtime_available: bool,
}

impl CapabilityAdvertisementEntry {
    pub fn new(
        capability_id: CapabilityId,
        min_version: CapabilityVersion,
        max_version: CapabilityVersion,
        runtime_available: bool,
    ) -> Result<Self, CapabilityAdvertisementError> {
        if min_version.major() != max_version.major() || min_version > max_version {
            return Err(CapabilityAdvertisementError::InvalidVersionRange);
        }

        Ok(Self {
            capability_id,
            min_version,
            max_version,
            runtime_available,
        })
    }

    pub fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
    }

    pub const fn min_version(&self) -> CapabilityVersion {
        self.min_version
    }

    pub const fn max_version(&self) -> CapabilityVersion {
        self.max_version
    }

    pub const fn runtime_available(&self) -> bool {
        self.runtime_available
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAdvertisement {
    entries: Vec<CapabilityAdvertisementEntry>,
}

impl CapabilityAdvertisement {
    pub fn new(
        entries: Vec<CapabilityAdvertisementEntry>,
    ) -> Result<Self, CapabilityAdvertisementError> {
        if entries.len() > MAX_CAPABILITY_ADVERTISEMENT_ENTRIES {
            return Err(CapabilityAdvertisementError::TooManyEntries);
        }

        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[CapabilityAdvertisementEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityAdvertisementError {
    InvalidVersionRange,
    TooManyEntries,
}

impl fmt::Display for CapabilityAdvertisementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidVersionRange => "capability advertisement version range is invalid",
            Self::TooManyEntries => "capability advertisement exceeds the entry limit",
        })
    }
}

impl std::error::Error for CapabilityAdvertisementError {}
