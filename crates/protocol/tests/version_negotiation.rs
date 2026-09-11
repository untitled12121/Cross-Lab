use crosslab_protocol::{
    ProtocolRange, ProtocolVersion, VersionNegotiationError, negotiate_protocol_version,
};

#[test]
fn negotiation_selects_highest_common_major_then_minor() {
    let local = [
        ProtocolRange::new(1, 0, 4).unwrap(),
        ProtocolRange::new(2, 0, 1).unwrap(),
    ];
    let peer = [
        ProtocolRange::new(1, 2, 6).unwrap(),
        ProtocolRange::new(2, 1, 3).unwrap(),
    ];

    assert_eq!(
        negotiate_protocol_version(&local, &peer).unwrap(),
        ProtocolVersion::new(2, 1)
    );
}

#[test]
fn negotiation_selects_highest_minor_in_overlap() {
    let local = [ProtocolRange::new(1, 1, 5).unwrap()];
    let peer = [ProtocolRange::new(1, 3, 7).unwrap()];

    assert_eq!(
        negotiate_protocol_version(&local, &peer).unwrap(),
        ProtocolVersion::new(1, 5)
    );
}

#[test]
fn invalid_range_is_rejected() {
    assert_eq!(
        ProtocolRange::new(1, 4, 3).unwrap_err(),
        VersionNegotiationError::InvalidRange
    );
}

#[test]
fn too_many_advertised_ranges_are_rejected() {
    let ranges = [ProtocolRange::new(1, 0, 0).unwrap(); 9];
    let peer = [ProtocolRange::new(1, 0, 0).unwrap()];

    assert_eq!(
        negotiate_protocol_version(&ranges, &peer).unwrap_err(),
        VersionNegotiationError::TooManyRanges
    );
}

#[test]
fn no_common_version_is_incompatible() {
    let local = [ProtocolRange::new(1, 0, 1).unwrap()];
    let peer = [ProtocolRange::new(2, 0, 1).unwrap()];

    assert_eq!(
        negotiate_protocol_version(&local, &peer).unwrap_err(),
        VersionNegotiationError::IncompatibleProtocol
    );
}
