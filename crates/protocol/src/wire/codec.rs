use core::fmt;

use crosslab_policy::{CapabilityId, CapabilityVersion, OperationId, OperationName, SessionId};
use prost::Message;

use crate::{
    DataStreamOpen, FrameError, FrameLimit, StreamDirection, StreamId, decode_frame, encode_frame,
};

use super::v1::{CapabilityVersionV1, DataStreamOpenV1, StreamDirectionV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolWireError {
    Frame(FrameError),
    MalformedProtobuf,
    InvalidProtocolVersion,
    InvalidSessionIdLength(usize),
    InvalidRequestIdLength(usize),
    InvalidEventIdLength(usize),
    InvalidStreamIdLength(usize),
    InvalidOperationIdLength(usize),
    InvalidPairingIdLength(usize),
    InvalidOwnerIdLength(usize),
    InvalidDeviceIdLength(usize),
    InvalidKeyIdLength(usize),
    InvalidDevicePublicKeyLength(usize),
    InvalidPairingNonceLength(usize),
    InvalidPairingConfirmationLength(usize),
    InvalidPairingTranscriptDigestLength(usize),
    InvalidSessionAuthNonceLength(usize),
    InvalidSessionAuthTranscriptDigestLength(usize),
    InvalidSignedObjectDigestLength(usize),
    InvalidSignatureLength(usize),
    MissingEnvelopeBody,
    MissingPairingBootstrapBody,
    MissingSessionAuthBootstrapBody,
    MissingDeviceCredential,
    MissingCapabilityVersion,
    MissingCapabilityMinVersion,
    MissingCapabilityMaxVersion,
    MissingControlResponseResult,
    InvalidCapabilityVersion,
    InvalidCapabilityVersionRange,
    InvalidCapabilityId,
    InvalidOperationName,
    InvalidEventType,
    InvalidEventScope,
    InvalidRetryClass(i32),
    InvalidProtocolErrorCode(u32),
    InvalidSessionCloseReason(i32),
    InvalidDiagnostic,
    InvalidStreamDirection(i32),
    InvalidPairingProfile(u32),
    InvalidPairingRole(i32),
    InvalidSessionAuthProfile(u32),
    InvalidSessionAuthRole(i32),
    InvalidSignatureAlgorithm(i32),
    InvalidCredentialSchema(u32),
    InvalidDevicePublicKey,
    InvalidDeviceCredential,
    CredentialIdentityMismatch,
    InvalidProtocolRange,
    InvalidFeatureId(u32),
    TooManyProtocolRanges(usize),
    TooManySupportedFeatures(usize),
    TooManyRequiredFeatures(usize),
    TooManyCapabilityEntries(usize),
}

impl fmt::Display for ProtocolWireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Frame(_) => "protocol frame is invalid",
            Self::MalformedProtobuf => "protobuf payload is malformed",
            Self::InvalidProtocolVersion => "protocol version is invalid",
            Self::InvalidSessionIdLength(_) => "session identifier length is invalid",
            Self::InvalidRequestIdLength(_) => "request identifier length is invalid",
            Self::InvalidEventIdLength(_) => "event identifier length is invalid",
            Self::InvalidStreamIdLength(_) => "stream identifier length is invalid",
            Self::InvalidOperationIdLength(_) => "operation identifier length is invalid",
            Self::InvalidPairingIdLength(_) => "pairing identifier length is invalid",
            Self::InvalidOwnerIdLength(_) => "owner identifier length is invalid",
            Self::InvalidDeviceIdLength(_) => "device identifier length is invalid",
            Self::InvalidKeyIdLength(_) => "key identifier length is invalid",
            Self::InvalidDevicePublicKeyLength(_) => "device public key length is invalid",
            Self::InvalidPairingNonceLength(_) => "pairing nonce length is invalid",
            Self::InvalidPairingConfirmationLength(_) => "pairing confirmation length is invalid",
            Self::InvalidPairingTranscriptDigestLength(_) => {
                "pairing transcript digest length is invalid"
            }
            Self::InvalidSessionAuthNonceLength(_) => {
                "session authentication nonce length is invalid"
            }
            Self::InvalidSessionAuthTranscriptDigestLength(_) => {
                "session authentication transcript digest length is invalid"
            }
            Self::InvalidSignedObjectDigestLength(_) => "signed object digest length is invalid",
            Self::InvalidSignatureLength(_) => "signature length is invalid",
            Self::MissingEnvelopeBody => "control envelope body is missing",
            Self::MissingPairingBootstrapBody => "pairing bootstrap body is missing",
            Self::MissingSessionAuthBootstrapBody => {
                "session authentication bootstrap body is missing"
            }
            Self::MissingDeviceCredential => "device credential is missing",
            Self::MissingCapabilityVersion => "capability version is missing",
            Self::MissingCapabilityMinVersion => "minimum capability version is missing",
            Self::MissingCapabilityMaxVersion => "maximum capability version is missing",
            Self::MissingControlResponseResult => "control response result is missing",
            Self::InvalidCapabilityVersion => "capability version is invalid",
            Self::InvalidCapabilityVersionRange => "capability version range is invalid",
            Self::InvalidCapabilityId => "capability identifier is invalid",
            Self::InvalidOperationName => "operation name is invalid",
            Self::InvalidEventType => "event type is invalid",
            Self::InvalidEventScope => "event scope is invalid",
            Self::InvalidRetryClass(_) => "retry class is invalid",
            Self::InvalidProtocolErrorCode(_) => "protocol error code is invalid",
            Self::InvalidSessionCloseReason(_) => "session close reason is invalid",
            Self::InvalidDiagnostic => "protocol diagnostic is invalid",
            Self::InvalidStreamDirection(_) => "stream direction is invalid",
            Self::InvalidPairingProfile(_) => "pairing profile is invalid",
            Self::InvalidPairingRole(_) => "pairing role is invalid",
            Self::InvalidSessionAuthProfile(_) => "session authentication profile is invalid",
            Self::InvalidSessionAuthRole(_) => "session authentication role is invalid",
            Self::InvalidSignatureAlgorithm(_) => "signature algorithm is invalid",
            Self::InvalidCredentialSchema(_) => "device credential schema is invalid",
            Self::InvalidDevicePublicKey => "device public key is invalid",
            Self::InvalidDeviceCredential => "device credential structure is invalid",
            Self::CredentialIdentityMismatch => {
                "session hello identity does not match its device credential"
            }
            Self::InvalidProtocolRange => "protocol range is invalid",
            Self::InvalidFeatureId(_) => "protocol feature identifier is invalid",
            Self::TooManyProtocolRanges(_) => "too many protocol ranges were advertised",
            Self::TooManySupportedFeatures(_) => "too many supported features were advertised",
            Self::TooManyRequiredFeatures(_) => "too many required features were advertised",
            Self::TooManyCapabilityEntries(_) => "capability advertisement exceeds the entry limit",
        })
    }
}

impl std::error::Error for ProtocolWireError {}

impl From<FrameError> for ProtocolWireError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

pub fn encode_data_stream_open(header: &DataStreamOpen) -> Result<Vec<u8>, ProtocolWireError> {
    let wire = DataStreamOpenV1::from(header);
    encode_frame(&wire.encode_to_vec(), FrameLimit::DataStreamOpen).map_err(Into::into)
}

pub fn decode_data_stream_open(frame: &[u8]) -> Result<DataStreamOpen, ProtocolWireError> {
    let payload = decode_frame(frame, FrameLimit::DataStreamOpen)?;
    let wire =
        DataStreamOpenV1::decode(payload).map_err(|_| ProtocolWireError::MalformedProtobuf)?;
    wire.try_into()
}

impl From<&DataStreamOpen> for DataStreamOpenV1 {
    fn from(header: &DataStreamOpen) -> Self {
        Self {
            session_id: header.session_id().to_bytes().to_vec(),
            stream_id: header.stream_id().to_bytes().to_vec(),
            operation_id: header.operation_id().to_bytes().to_vec(),
            capability_id: header.capability_id().as_str().to_owned(),
            capability_version: Some(CapabilityVersionV1 {
                major: header.capability_version().major().into(),
                minor: header.capability_version().minor().into(),
            }),
            operation_name: header.operation_name().as_str().to_owned(),
            direction: match header.direction() {
                StreamDirection::SourceToDestination => {
                    StreamDirectionV1::SourceToDestination as i32
                }
                StreamDirection::DestinationToSource => {
                    StreamDirectionV1::DestinationToSource as i32
                }
            },
            stream_index: header.stream_index(),
        }
    }
}

impl TryFrom<DataStreamOpenV1> for DataStreamOpen {
    type Error = ProtocolWireError;

    fn try_from(wire: DataStreamOpenV1) -> Result<Self, Self::Error> {
        let session_id = SessionId::from_bytes(copy_32(
            wire.session_id,
            ProtocolWireError::InvalidSessionIdLength,
        )?);
        let stream_id = StreamId::from_bytes(copy_16(
            wire.stream_id,
            ProtocolWireError::InvalidStreamIdLength,
        )?);
        let operation_id = OperationId::from_bytes(copy_32(
            wire.operation_id,
            ProtocolWireError::InvalidOperationIdLength,
        )?);
        let capability_id = CapabilityId::parse(&wire.capability_id)
            .map_err(|_| ProtocolWireError::InvalidCapabilityId)?;
        let capability_version = wire
            .capability_version
            .ok_or(ProtocolWireError::MissingCapabilityVersion)
            .and_then(capability_version_from_wire)?;
        let operation_name = OperationName::parse(&wire.operation_name)
            .map_err(|_| ProtocolWireError::InvalidOperationName)?;
        let direction = match wire.direction {
            value if value == StreamDirectionV1::SourceToDestination as i32 => {
                StreamDirection::SourceToDestination
            }
            value if value == StreamDirectionV1::DestinationToSource as i32 => {
                StreamDirection::DestinationToSource
            }
            value => return Err(ProtocolWireError::InvalidStreamDirection(value)),
        };

        Ok(Self::new(
            session_id,
            stream_id,
            operation_id,
            capability_id,
            capability_version,
            operation_name,
            direction,
            wire.stream_index,
        ))
    }
}

pub(super) fn capability_version_from_wire(
    wire: CapabilityVersionV1,
) -> Result<CapabilityVersion, ProtocolWireError> {
    let major =
        u16::try_from(wire.major).map_err(|_| ProtocolWireError::InvalidCapabilityVersion)?;
    let minor =
        u16::try_from(wire.minor).map_err(|_| ProtocolWireError::InvalidCapabilityVersion)?;
    Ok(CapabilityVersion::new(major, minor))
}

pub(super) fn copy_16(
    bytes: Vec<u8>,
    error: fn(usize) -> ProtocolWireError,
) -> Result<[u8; 16], ProtocolWireError> {
    let len = bytes.len();
    bytes.try_into().map_err(|_| error(len))
}

pub(super) fn copy_32(
    bytes: Vec<u8>,
    error: fn(usize) -> ProtocolWireError,
) -> Result<[u8; 32], ProtocolWireError> {
    let len = bytes.len();
    bytes.try_into().map_err(|_| error(len))
}

pub(super) fn copy_64(
    bytes: Vec<u8>,
    error: fn(usize) -> ProtocolWireError,
) -> Result<[u8; 64], ProtocolWireError> {
    let len = bytes.len();
    bytes.try_into().map_err(|_| error(len))
}
