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
pub struct EventV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub event_id: Vec<u8>,
    #[prost(string, optional, tag = "2")]
    pub capability_id: Option<String>,
    #[prost(string, tag = "3")]
    pub event_type: String,
    #[prost(bytes = "vec", tag = "4")]
    pub body: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct CancelRequestV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub request_id: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum SessionCloseReasonV1 {
    Unspecified = 0,
    Normal = 1,
    LocalRequest = 2,
    ProtocolError = 3,
    AuthenticationLost = 4,
    TrustRevoked = 5,
    Shutdown = 6,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct SessionCloseV1 {
    #[prost(enumeration = "SessionCloseReasonV1", tag = "1")]
    pub reason: i32,
    #[prost(string, optional, tag = "2")]
    pub diagnostic: Option<String>,
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
    #[prost(oneof = "envelope_v1::Body", tags = "5, 6, 7, 8, 9, 10, 11")]
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
        #[prost(message, tag = "8")]
        Event(super::EventV1),
        #[prost(message, tag = "9")]
        CancelRequest(super::CancelRequestV1),
        #[prost(message, tag = "10")]
        ProtocolError(super::ProtocolErrorV1),
        #[prost(message, tag = "11")]
        SessionClose(super::SessionCloseV1),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum PairingRoleV1 {
    Unspecified = 0,
    Inviter = 1,
    Joiner = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum SignatureAlgorithmV1 {
    Unspecified = 0,
    Ed25519 = 1,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct PairingHelloV1 {
    #[prost(enumeration = "PairingRoleV1", tag = "1")]
    pub role: i32,
    #[prost(uint32, tag = "2")]
    pub protocol_major: u32,
    #[prost(bytes = "vec", tag = "3")]
    pub pairing_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub owner_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub device_id: Vec<u8>,
    #[prost(enumeration = "SignatureAlgorithmV1", tag = "6")]
    pub device_algorithm: i32,
    #[prost(bytes = "vec", tag = "7")]
    pub device_public_key: Vec<u8>,
    #[prost(bytes = "vec", tag = "8")]
    pub nonce: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct PairingConfirmationV1 {
    #[prost(enumeration = "PairingRoleV1", tag = "1")]
    pub role: i32,
    #[prost(bytes = "vec", tag = "2")]
    pub pairing_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub confirmation: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct PairingCredentialAcceptedV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub pairing_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub pairing_transcript_digest: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub device_credential_signed_object_digest: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub joiner_device_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub joiner_device_key_id: Vec<u8>,
    #[prost(enumeration = "SignatureAlgorithmV1", tag = "6")]
    pub signature_algorithm: i32,
    #[prost(bytes = "vec", tag = "7")]
    pub signature: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct PairingBootstrapV1 {
    #[prost(uint32, tag = "1")]
    pub pairing_profile: u32,
    #[prost(oneof = "pairing_bootstrap_v1::Body", tags = "2, 3, 4")]
    pub body: Option<pairing_bootstrap_v1::Body>,
}

pub mod pairing_bootstrap_v1 {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Body {
        #[prost(message, tag = "2")]
        Hello(super::PairingHelloV1),
        #[prost(message, tag = "3")]
        Confirmation(super::PairingConfirmationV1),
        #[prost(message, tag = "4")]
        CredentialAccepted(super::PairingCredentialAcceptedV1),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum AuthorityRoleV1 {
    Unspecified = 0,
    DeviceSigning = 1,
    Administrative = 2,
    Recovery = 3,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct OwnerRootRecordV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub owner_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub root_key_id: Vec<u8>,
    #[prost(enumeration = "SignatureAlgorithmV1", tag = "4")]
    pub root_algorithm: i32,
    #[prost(bytes = "vec", tag = "5")]
    pub root_public_key: Vec<u8>,
    #[prost(uint64, tag = "6")]
    pub root_epoch: u64,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AuthorityDelegationV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub owner_id: Vec<u8>,
    #[prost(enumeration = "AuthorityRoleV1", tag = "3")]
    pub role: i32,
    #[prost(bytes = "vec", tag = "4")]
    pub delegated_key_id: Vec<u8>,
    #[prost(enumeration = "SignatureAlgorithmV1", tag = "5")]
    pub delegated_algorithm: i32,
    #[prost(bytes = "vec", tag = "6")]
    pub delegated_public_key: Vec<u8>,
    #[prost(uint64, tag = "7")]
    pub delegation_epoch: u64,
    #[prost(bytes = "vec", tag = "8")]
    pub issuer_root_key_id: Vec<u8>,
    #[prost(enumeration = "SignatureAlgorithmV1", tag = "9")]
    pub signature_algorithm: i32,
    #[prost(bytes = "vec", tag = "10")]
    pub signature: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct PairingTrustTransitionV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub owner_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub device_id: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub credential_epoch: u64,
    #[prost(bytes = "vec", tag = "5")]
    pub credential_signed_object_digest: Vec<u8>,
    #[prost(bytes = "vec", tag = "6")]
    pub transition_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "7")]
    pub pairing_evidence_digest: Vec<u8>,
    #[prost(bytes = "vec", tag = "8")]
    pub issuer_key_id: Vec<u8>,
    #[prost(enumeration = "SignatureAlgorithmV1", tag = "9")]
    pub signature_algorithm: i32,
    #[prost(bytes = "vec", tag = "10")]
    pub signature: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProductPairingCredentialBundleV1 {
    #[prost(message, optional, tag = "1")]
    pub owner_root: Option<OwnerRootRecordV1>,
    #[prost(message, optional, tag = "2")]
    pub device_signing: Option<AuthorityDelegationV1>,
    #[prost(message, optional, tag = "3")]
    pub credential: Option<DeviceCredentialV1>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProductPairingTrustBundleV1 {
    #[prost(message, optional, tag = "1")]
    pub credential: Option<DeviceCredentialV1>,
    #[prost(message, optional, tag = "2")]
    pub transition: Option<PairingTrustTransitionV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum PairingAckKindV1 {
    Unspecified = 0,
    Persisted = 1,
    Complete = 2,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct PairingAckV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub pairing_id: Vec<u8>,
    #[prost(enumeration = "PairingRoleV1", tag = "2")]
    pub role: i32,
    #[prost(enumeration = "PairingAckKindV1", tag = "3")]
    pub kind: i32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct PairingCancelV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub pairing_id: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProductPairingV1 {
    #[prost(uint32, tag = "1")]
    pub pairing_profile: u32,
    #[prost(oneof = "product_pairing_v1::Body", tags = "2, 3, 4, 5, 6, 7, 8")]
    pub body: Option<product_pairing_v1::Body>,
}

pub mod product_pairing_v1 {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Body {
        #[prost(message, tag = "2")]
        Hello(super::PairingHelloV1),
        #[prost(message, tag = "3")]
        Confirmation(super::PairingConfirmationV1),
        #[prost(message, tag = "4")]
        CredentialBundle(super::ProductPairingCredentialBundleV1),
        #[prost(message, tag = "5")]
        CredentialAccepted(super::PairingCredentialAcceptedV1),
        #[prost(message, tag = "6")]
        TrustBundle(super::ProductPairingTrustBundleV1),
        #[prost(message, tag = "7")]
        Ack(super::PairingAckV1),
        #[prost(message, tag = "8")]
        Cancel(super::PairingCancelV1),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum SessionAuthRoleV1 {
    Unspecified = 0,
    Initiator = 1,
    Responder = 2,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ProtocolRangeV1 {
    #[prost(uint32, tag = "1")]
    pub major: u32,
    #[prost(uint32, tag = "2")]
    pub min_minor: u32,
    #[prost(uint32, tag = "3")]
    pub max_minor: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct DeviceCredentialV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub owner_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub device_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub device_key_id: Vec<u8>,
    #[prost(enumeration = "SignatureAlgorithmV1", tag = "5")]
    pub device_algorithm: i32,
    #[prost(bytes = "vec", tag = "6")]
    pub device_public_key: Vec<u8>,
    #[prost(uint64, tag = "7")]
    pub credential_epoch: u64,
    #[prost(bytes = "vec", tag = "8")]
    pub issuer_device_signing_key_id: Vec<u8>,
    #[prost(enumeration = "SignatureAlgorithmV1", tag = "9")]
    pub signature_algorithm: i32,
    #[prost(bytes = "vec", tag = "10")]
    pub signature: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct SessionAuthHelloV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub owner_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub device_id: Vec<u8>,
    #[prost(message, optional, tag = "3")]
    pub device_credential: Option<DeviceCredentialV1>,
    #[prost(message, repeated, tag = "4")]
    pub protocol_ranges: Vec<ProtocolRangeV1>,
    #[prost(uint32, repeated, tag = "5")]
    pub supported_features: Vec<u32>,
    #[prost(uint32, repeated, tag = "6")]
    pub required_features: Vec<u32>,
    #[prost(bytes = "vec", tag = "7")]
    pub nonce: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct SessionAuthProofV1 {
    #[prost(enumeration = "SessionAuthRoleV1", tag = "1")]
    pub role: i32,
    #[prost(bytes = "vec", tag = "2")]
    pub transcript_digest: Vec<u8>,
    #[prost(enumeration = "SignatureAlgorithmV1", tag = "3")]
    pub signature_algorithm: i32,
    #[prost(bytes = "vec", tag = "4")]
    pub signature: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct SessionAuthBootstrapV1 {
    #[prost(uint32, tag = "1")]
    pub session_auth_profile: u32,
    #[prost(oneof = "session_auth_bootstrap_v1::Body", tags = "2, 3")]
    pub body: Option<session_auth_bootstrap_v1::Body>,
}

pub mod session_auth_bootstrap_v1 {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Body {
        #[prost(message, boxed, tag = "2")]
        Hello(Box<super::SessionAuthHelloV1>),
        #[prost(message, tag = "3")]
        Proof(super::SessionAuthProofV1),
    }
}
