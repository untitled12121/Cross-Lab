//! Versioned Cross-Lab wire-contract domain.

mod capability;
mod control;
mod data_stream;
mod feature;
mod file_transfer;
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
pub use file_transfer::{
    FILE_TRANSFER_CAPABILITY_ID, FILE_TRANSFER_CHECKPOINT_BYTES, FILE_TRANSFER_PROFILE_V2,
    FILE_TRANSFER_RESULT_EVENT_TYPE, FileTransferAcceptance, FileTransferDigest, FileTransferOffer,
    FileTransferProfileError, FileTransferResult, FileTransferTerminalOutcome,
    MAX_FILE_TRANSFER_DISPLAY_NAME_BYTES, TransferId, TransferIdGenerationError,
    valid_resume_offset,
};
pub use framing::{FrameError, FrameLimit, decode_frame, encode_frame};
pub use version::{
    MAX_PROTOCOL_RANGES, ProtocolRange, ProtocolVersion, VersionNegotiationError,
    negotiate_protocol_version,
};
pub use wire::{
    FileTransferWireError, MAX_FILE_TRANSFER_ACCEPTANCE_WIRE_BYTES,
    MAX_FILE_TRANSFER_OFFER_WIRE_BYTES, MAX_FILE_TRANSFER_RESULT_WIRE_BYTES, PAIRING_PROFILE_V1,
    PairingBootstrapMessage, PairingConfirmation, PairingCredentialAccepted, PairingHello,
    PairingRole, ProductPairingAck, ProductPairingAckKind, ProductPairingCredentialBundle,
    ProductPairingMessage, ProductPairingTrustBundleMessage, ProtocolWireError,
    SESSION_AUTH_PROFILE_V1, SessionAuthBootstrapMessage, SessionAuthHello,
    SessionAuthProofMessage, SessionAuthRole, decode_control_envelope, decode_data_stream_open,
    decode_file_transfer_acceptance, decode_file_transfer_offer, decode_file_transfer_result,
    decode_pairing_bootstrap, decode_product_pairing, decode_session_auth_bootstrap,
    encode_control_envelope, encode_data_stream_open, encode_file_transfer_acceptance,
    encode_file_transfer_offer, encode_file_transfer_result, encode_pairing_bootstrap,
    encode_product_pairing, encode_session_auth_bootstrap,
};
