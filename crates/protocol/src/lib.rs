//! Versioned Cross-Lab wire-contract domain.

mod control;
mod feature;
mod framing;
mod version;

pub use control::{ControlSequence, SequenceError};
pub use feature::{
    FeatureNegotiationError, FeatureSet, MAX_REQUIRED_FEATURES, MAX_SUPPORTED_FEATURES,
    negotiate_features,
};
pub use framing::{FrameError, FrameLimit, decode_frame, encode_frame};
pub use version::{
    MAX_PROTOCOL_RANGES, ProtocolRange, ProtocolVersion, VersionNegotiationError,
    negotiate_protocol_version,
};
