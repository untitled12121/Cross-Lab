//! Versioned Cross-Lab wire-contract domain.

mod capability;
mod control;
mod data_stream;
mod feature;
mod framing;
mod version;
pub mod wire;

pub use capability::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, CapabilityAdvertisementError,
    MAX_CAPABILITY_ADVERTISEMENT_ENTRIES,
};
pub use control::{
    CancelRequest, ControlEnvelope, ControlRequest, ControlResponse, ControlResponseResult,
    ControlSequence, DiagnosticError, EnvelopeBody, Event, EventError, EventId, EventScope,
    EventType, EventTypeError, MAX_DIAGNOSTIC_BYTES, MAX_EVENT_TYPE_BYTES, ProtocolDiagnostic,
    ProtocolErrorCode, ProtocolErrorCodeError, ProtocolFailure, ProtocolIdError, RequestId,
    RetryClass, RetryClassError, SYSTEM_EVENT_TYPE_PREFIX, SequenceError, SessionClose,
    SessionCloseReason, SessionCloseReasonError, StreamId,
};
pub use data_stream::{DataStreamOpen, StreamDirection};
pub use feature::{
    FeatureNegotiationError, FeatureSet, MAX_REQUIRED_FEATURES, MAX_SUPPORTED_FEATURES,
    negotiate_features,
};
pub use framing::{FrameError, FrameLimit, decode_frame, encode_frame};
pub use version::{
    MAX_PROTOCOL_RANGES, ProtocolRange, ProtocolVersion, VersionNegotiationError,
    negotiate_protocol_version,
};
pub use wire::{
    PAIRING_PROFILE_V1, PairingBootstrapMessage, PairingConfirmation, PairingCredentialAccepted,
    PairingHello, PairingRole, ProtocolWireError, decode_control_envelope, decode_data_stream_open,
    decode_pairing_bootstrap, encode_control_envelope, encode_data_stream_open,
    encode_pairing_bootstrap,
};
