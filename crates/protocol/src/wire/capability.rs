use crosslab_policy::CapabilityId;

use crate::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, CapabilityAdvertisementError,
    MAX_CAPABILITY_ADVERTISEMENT_ENTRIES,
};

use super::{
    codec::{ProtocolWireError, capability_version_from_wire},
    v1::{CapabilityAdvertisementEntryV1, CapabilityAdvertisementV1, CapabilityVersionV1},
};

impl From<&CapabilityAdvertisement> for CapabilityAdvertisementV1 {
    fn from(advertisement: &CapabilityAdvertisement) -> Self {
        Self {
            entries: advertisement
                .entries()
                .iter()
                .map(CapabilityAdvertisementEntryV1::from)
                .collect(),
        }
    }
}

impl From<&CapabilityAdvertisementEntry> for CapabilityAdvertisementEntryV1 {
    fn from(entry: &CapabilityAdvertisementEntry) -> Self {
        Self {
            capability_id: entry.capability_id().as_str().to_owned(),
            min_version: Some(CapabilityVersionV1 {
                major: entry.min_version().major().into(),
                minor: entry.min_version().minor().into(),
            }),
            max_version: Some(CapabilityVersionV1 {
                major: entry.max_version().major().into(),
                minor: entry.max_version().minor().into(),
            }),
            runtime_available: entry.runtime_available(),
        }
    }
}

impl TryFrom<CapabilityAdvertisementV1> for CapabilityAdvertisement {
    type Error = ProtocolWireError;

    fn try_from(wire: CapabilityAdvertisementV1) -> Result<Self, Self::Error> {
        if wire.entries.len() > MAX_CAPABILITY_ADVERTISEMENT_ENTRIES {
            return Err(ProtocolWireError::TooManyCapabilityEntries(
                wire.entries.len(),
            ));
        }

        let entries = wire
            .entries
            .into_iter()
            .map(CapabilityAdvertisementEntry::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        CapabilityAdvertisement::new(entries).map_err(|error| match error {
            CapabilityAdvertisementError::TooManyEntries => {
                ProtocolWireError::TooManyCapabilityEntries(
                    MAX_CAPABILITY_ADVERTISEMENT_ENTRIES + 1,
                )
            }
            CapabilityAdvertisementError::InvalidVersionRange => {
                ProtocolWireError::InvalidCapabilityVersionRange
            }
        })
    }
}

impl TryFrom<CapabilityAdvertisementEntryV1> for CapabilityAdvertisementEntry {
    type Error = ProtocolWireError;

    fn try_from(wire: CapabilityAdvertisementEntryV1) -> Result<Self, Self::Error> {
        let capability_id = CapabilityId::parse(&wire.capability_id)
            .map_err(|_| ProtocolWireError::InvalidCapabilityId)?;
        let min_version = wire
            .min_version
            .ok_or(ProtocolWireError::MissingCapabilityMinVersion)
            .and_then(capability_version_from_wire)?;
        let max_version = wire
            .max_version
            .ok_or(ProtocolWireError::MissingCapabilityMaxVersion)
            .and_then(capability_version_from_wire)?;

        CapabilityAdvertisementEntry::new(
            capability_id,
            min_version,
            max_version,
            wire.runtime_available,
        )
        .map_err(|_| ProtocolWireError::InvalidCapabilityVersionRange)
    }
}
