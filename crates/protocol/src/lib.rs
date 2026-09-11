//! Versioned Cross-Lab wire-contract domain.

mod capability;
mod control;
mod feature;
mod framing;
mod version;

pub use capability::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, CapabilityAdvertisementError,
    MAX_CAPABILITY_ADVERTISEMENT_ENTRIES,
};
pub use control::{
    ControlSequence, DiagnosticError, EventId, MAX_DIAGNOSTIC_BYTES, ProtocolDiagnostic,
    ProtocolIdError, RequestId, RetryClass, RetryClassError, SequenceError, StreamId,
};
pub use feature::{
    FeatureNegotiationError, FeatureSet, MAX_REQUIRED_FEATURES, MAX_SUPPORTED_FEATURES,
    negotiate_features,
};
pub use framing::{FrameError, FrameLimit, decode_frame, encode_frame};
pub use version::{
    MAX_PROTOCOL_RANGES, ProtocolRange, ProtocolVersion, VersionNegotiationError,
    negotiate_protocol_version,
};
