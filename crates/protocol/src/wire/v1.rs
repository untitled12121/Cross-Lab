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
