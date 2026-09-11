use crosslab_policy::{CapabilityId, CapabilityVersion};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, CapabilityAdvertisementError,
};

fn entry(id: &str, min_minor: u16, max_minor: u16) -> CapabilityAdvertisementEntry {
    CapabilityAdvertisementEntry::new(
        CapabilityId::parse(id).unwrap(),
        CapabilityVersion::new(1, min_minor),
        CapabilityVersion::new(1, max_minor),
        true,
    )
    .unwrap()
}

#[test]
fn advertisement_entry_preserves_validated_capability_state() {
    let entry = CapabilityAdvertisementEntry::new(
        CapabilityId::parse("files.transfer").unwrap(),
        CapabilityVersion::new(1, 0),
        CapabilityVersion::new(1, 3),
        false,
    )
    .unwrap();

    assert_eq!(entry.capability_id().as_str(), "files.transfer");
    assert_eq!(entry.min_version(), CapabilityVersion::new(1, 0));
    assert_eq!(entry.max_version(), CapabilityVersion::new(1, 3));
    assert!(!entry.runtime_available());
}

#[test]
fn advertisement_entry_rejects_incompatible_version_window() {
    assert_eq!(
        CapabilityAdvertisementEntry::new(
            CapabilityId::parse("files.transfer").unwrap(),
            CapabilityVersion::new(1, 3),
            CapabilityVersion::new(1, 2),
            true,
        )
        .unwrap_err(),
        CapabilityAdvertisementError::InvalidVersionRange
    );
    assert_eq!(
        CapabilityAdvertisementEntry::new(
            CapabilityId::parse("files.transfer").unwrap(),
            CapabilityVersion::new(1, 0),
            CapabilityVersion::new(2, 0),
            true,
        )
        .unwrap_err(),
        CapabilityAdvertisementError::InvalidVersionRange
    );
}

#[test]
fn advertisement_is_bounded_to_256_entries() {
    let entries = (0..256)
        .map(|index| entry(&format!("test.cap-{index}"), 0, 0))
        .collect::<Vec<_>>();
    assert_eq!(CapabilityAdvertisement::new(entries).unwrap().len(), 256);

    let oversized = (0..257)
        .map(|index| entry(&format!("test.cap-{index}"), 0, 0))
        .collect::<Vec<_>>();
    assert_eq!(
        CapabilityAdvertisement::new(oversized).unwrap_err(),
        CapabilityAdvertisementError::TooManyEntries
    );
}
