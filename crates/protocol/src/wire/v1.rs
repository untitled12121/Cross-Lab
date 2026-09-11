#[derive(Clone, PartialEq, prost::Message)]
pub struct CapabilityVersionV1 {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint32, tag = "2")]
    pub minor: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct CapabilityAdvertisementEntryV1 {
    #[prost(string, tag = "1")]
    pub capability_id: String,
    #[prost(message, optional, tag = "2")]
    pub min_version: Option<CapabilityVersionV1>,
    #[prost(message, optional, tag = "3")]
    pub max_version: Option<CapabilityVersionV1>,
    #[prost(bool, tag = "4")]
    pub runtime_available: bool,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct CapabilityAdvertisementV1 {
    #[prost(message, repeated, tag = "1")]
    pub entries: Vec<CapabilityAdvertisementEntryV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum RetryClassV1 {
    NonRetryable = 0,
    Idempotent = 1,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ControlRequestV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub request_id: Vec<u8>,
    #[prost(string, tag = "2")]
    pub capability_id: String,
    #[prost(message, optional, tag = "3")]
    pub capability_version: Option<CapabilityVersionV1>,
    #[prost(string, tag = "4")]
    pub operation_name: String,
    #[prost(enumeration = "RetryClassV1", tag = "5")]
    pub retry_class: i32,
    #[prost(bytes = "vec", tag = "6")]
    pub body: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProtocolErrorV1 {
    #[prost(uint32, tag = "1")]
    pub code: u32,
    #[prost(string, optional, tag = "2")]
    pub diagnostic: Option<String>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ControlResponseV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub request_id: Vec<u8>,
    #[prost(oneof = "control_response_v1::Result", tags = "2, 3")]
    pub result: Option<control_response_v1::Result>,
}

pub mod control_response_v1 {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Result {
        #[prost(bytes, tag = "2")]
        SuccessBody(Vec<u8>),
        #[prost(message, tag = "3")]
        Error(super::ProtocolErrorV1),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct CancelRequestV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub request_id: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct EnvelopeV1 {
    #[prost(uint32, tag = "1")]
    pub protocol_major: u32,
    #[prost(uint32, tag = "2")]
    pub protocol_minor: u32,
    #[prost(bytes = "vec", tag = "3")]
    pub session_id: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub message_seq: u64,
    #[prost(oneof = "envelope_v1::Body", tags = "5, 6, 7, 9, 10")]
    pub body: Option<envelope_v1::Body>,
}

pub mod envelope_v1 {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Body {
        #[prost(message, tag = "5")]
        CapabilityAdvertisement(super::CapabilityAdvertisementV1),
        #[prost(message, tag = "6")]
        ControlRequest(super::ControlRequestV1),
        #[prost(message, tag = "7")]
        ControlResponse(super::ControlResponseV1),
        #[prost(message, tag = "9")]
        CancelRequest(super::CancelRequestV1),
        #[prost(message, tag = "10")]
        ProtocolError(super::ProtocolErrorV1),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct DataStreamOpenV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub session_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub stream_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub operation_id: Vec<u8>,
    #[prost(string, tag = "4")]
    pub capability_id: String,
    #[prost(message, optional, tag = "5")]
    pub capability_version: Option<CapabilityVersionV1>,
    #[prost(string, tag = "6")]
    pub operation_name: String,
    #[prost(enumeration = "StreamDirectionV1", tag = "7")]
    pub direction: i32,
    #[prost(uint32, tag = "8")]
    pub stream_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum StreamDirectionV1 {
    Unspecified = 0,
    SourceToDestination = 1,
    DestinationToSource = 2,
}
