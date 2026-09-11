use crosslab_policy::{CapabilityId, CapabilityVersion, OperationId, OperationName, SessionId};
use crosslab_protocol::{DataStreamOpen, StreamDirection, StreamId};

fn open_header(direction: StreamDirection, stream_index: u32) -> DataStreamOpen {
    DataStreamOpen::new(
        SessionId::from_bytes([1; 32]),
        StreamId::from_bytes([2; 16]),
        OperationId::from_bytes([3; 32]),
        CapabilityId::parse("files.transfer").unwrap(),
        CapabilityVersion::new(1, 2),
        OperationName::parse("send").unwrap(),
        direction,
        stream_index,
    )
}

#[test]
fn data_stream_open_preserves_authorization_bindings() {
    let header = open_header(StreamDirection::SourceToDestination, 7);

    assert_eq!(header.session_id().to_bytes(), [1; 32]);
    assert_eq!(header.stream_id().to_bytes(), [2; 16]);
    assert_eq!(header.operation_id().to_bytes(), [3; 32]);
    assert_eq!(header.capability_id().as_str(), "files.transfer");
    assert_eq!(header.capability_version(), CapabilityVersion::new(1, 2));
    assert_eq!(header.operation_name().as_str(), "send");
    assert_eq!(header.direction(), StreamDirection::SourceToDestination);
    assert_eq!(header.stream_index(), 7);
}

#[test]
fn stream_direction_is_explicit_relative_to_operation_endpoints() {
    assert_ne!(
        StreamDirection::SourceToDestination,
        StreamDirection::DestinationToSource
    );
}
