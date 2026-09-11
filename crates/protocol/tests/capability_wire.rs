use crosslab_policy::{CapabilityId, CapabilityVersion};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, MAX_CAPABILITY_ADVERTISEMENT_ENTRIES,
    ProtocolWireError, wire,
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

fn wire_entry(id: &str) -> wire::v1::CapabilityAdvertisementEntryV1 {
    wire::v1::CapabilityAdvertisementEntryV1 {
        capability_id: id.into(),
        min_version: Some(wire::v1::CapabilityVersionV1 { major: 1, minor: 0 }),
        max_version: Some(wire::v1::CapabilityVersionV1 { major: 1, minor: 2 }),
        runtime_available: true,
    }
}

#[test]
fn capability_advertisement_round_trips_through_strict_wire_conversion() {
    let expected = CapabilityAdvertisement::new(vec![
        entry("clipboard.read", 0, 0),
        entry("files.transfer", 0, 2),
    ])
    .unwrap();

    let wire = wire::v1::CapabilityAdvertisementV1::from(&expected);
    let decoded = CapabilityAdvertisement::try_from(wire).unwrap();

    assert_eq!(decoded, expected);
}

#[test]
fn wire_capability_advertisement_rejects_oversized_collection_before_conversion() {
    let wire = wire::v1::CapabilityAdvertisementV1 {
        entries: (0..=MAX_CAPABILITY_ADVERTISEMENT_ENTRIES)
            .map(|index| wire_entry(&format!("test.cap-{index}")))
            .collect(),
    };

    assert_eq!(
        CapabilityAdvertisement::try_from(wire).unwrap_err(),
        ProtocolWireError::TooManyCapabilityEntries(MAX_CAPABILITY_ADVERTISEMENT_ENTRIES + 1)
    );
}

#[test]
fn wire_capability_advertisement_rejects_invalid_domain_identifier() {
    let wire = wire::v1::CapabilityAdvertisementV1 {
        entries: vec![wire_entry("FILES.TRANSFER")],
    };

    assert_eq!(
        CapabilityAdvertisement::try_from(wire).unwrap_err(),
        ProtocolWireError::InvalidCapabilityId
    );
}
