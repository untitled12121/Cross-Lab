use crosslab_policy::{CapabilityId, CapabilityVersion, OperationId, OperationName, SessionId};
use crosslab_protocol::{
    DataStreamOpen, FrameLimit, ProtocolWireError, StreamDirection, StreamId,
    decode_data_stream_open, encode_data_stream_open, encode_frame, wire,
};
use prost::Message;

fn header() -> DataStreamOpen {
    DataStreamOpen::new(
        SessionId::from_bytes([1; 32]),
        StreamId::from_bytes([2; 16]),
        OperationId::from_bytes([3; 32]),
        CapabilityId::parse("files.transfer").unwrap(),
        CapabilityVersion::new(1, 2),
        OperationName::parse("send").unwrap(),
        StreamDirection::SourceToDestination,
        7,
    )
}

#[test]
fn data_stream_open_round_trips_through_protobuf_and_bounded_framing() {
    let expected = header();
    let encoded = encode_data_stream_open(&expected).unwrap();

    assert_eq!(decode_data_stream_open(&encoded).unwrap(), expected);
}

#[test]
fn malformed_fixed_length_identifiers_are_rejected_during_domain_conversion() {
    let malformed = wire::v1::DataStreamOpenV1 {
        session_id: vec![1; 31],
        stream_id: vec![2; 16],
        operation_id: vec![3; 32],
        capability_id: "files.transfer".into(),
        capability_version: Some(wire::v1::CapabilityVersionV1 { major: 1, minor: 2 }),
        operation_name: "send".into(),
        direction: wire::v1::StreamDirectionV1::SourceToDestination as i32,
        stream_index: 7,
    };
    let frame = encode_frame(&malformed.encode_to_vec(), FrameLimit::DataStreamOpen).unwrap();

    assert_eq!(
        decode_data_stream_open(&frame).unwrap_err(),
        ProtocolWireError::InvalidSessionIdLength(31)
    );
}
