use core::fmt;

use crosslab_policy::OperationId;
use prost::Message;

use crate::{
    FILE_TRANSFER_PROFILE_V2, FileTransferAcceptance, FileTransferDigest, FileTransferOffer,
    FileTransferResult, FileTransferTerminalOutcome, TransferId,
};

use super::v1::{
    FileTransferAcceptanceV2, FileTransferAlreadyCompleteV2, FileTransferOfferV2,
    FileTransferReadyV2, FileTransferResultV2, FileTransferTerminalOutcomeV2,
    file_transfer_acceptance_v2,
};

pub const MAX_FILE_TRANSFER_OFFER_WIRE_BYTES: usize = 384;
pub const MAX_FILE_TRANSFER_ACCEPTANCE_WIRE_BYTES: usize = 128;
pub const MAX_FILE_TRANSFER_RESULT_WIRE_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferWireError {
    PayloadTooLarge { actual: usize, max: usize },
    MalformedProtobuf,
    InvalidProfile(u32),
    InvalidTransferIdLength(usize),
    InvalidDigestLength(usize),
    InvalidDisplayName,
    MissingAcceptance,
    InvalidOperationIdLength(usize),
    InvalidTerminalOutcome(i32),
}

impl fmt::Display for FileTransferWireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PayloadTooLarge { .. } => "file transfer payload exceeds its profile limit",
            Self::MalformedProtobuf => "file transfer protobuf payload is malformed",
            Self::InvalidProfile(_) => "file transfer profile version is invalid",
            Self::InvalidTransferIdLength(_) => "file transfer identifier length is invalid",
            Self::InvalidDigestLength(_) => "file transfer digest length is invalid",
            Self::InvalidDisplayName => "file transfer display name is invalid",
            Self::MissingAcceptance => "file transfer acceptance result is missing",
            Self::InvalidOperationIdLength(_) => {
                "file transfer operation identifier length is invalid"
            }
            Self::InvalidTerminalOutcome(_) => "file transfer terminal outcome is invalid",
        })
    }
}

impl std::error::Error for FileTransferWireError {}

pub fn encode_file_transfer_offer(
    offer: &FileTransferOffer,
) -> Result<Vec<u8>, FileTransferWireError> {
    encode_bounded(
        &FileTransferOfferV2 {
            profile_version: FILE_TRANSFER_PROFILE_V2,
            transfer_id: offer.transfer_id().to_bytes().to_vec(),
            display_name: offer.display_name().to_owned(),
            file_size: offer.file_size(),
            blake3_digest: offer.digest().to_bytes().to_vec(),
        },
        MAX_FILE_TRANSFER_OFFER_WIRE_BYTES,
    )
}

pub fn decode_file_transfer_offer(
    payload: &[u8],
) -> Result<FileTransferOffer, FileTransferWireError> {
    let wire: FileTransferOfferV2 = decode_bounded(payload, MAX_FILE_TRANSFER_OFFER_WIRE_BYTES)?;
    require_profile(wire.profile_version)?;

    FileTransferOffer::new(
        decode_transfer_id(wire.transfer_id)?,
        wire.display_name,
        wire.file_size,
        FileTransferDigest::from_bytes(copy_32(
            wire.blake3_digest,
            FileTransferWireError::InvalidDigestLength,
        )?),
    )
    .map_err(|_| FileTransferWireError::InvalidDisplayName)
}

pub fn encode_file_transfer_acceptance(
    acceptance: &FileTransferAcceptance,
) -> Result<Vec<u8>, FileTransferWireError> {
    let result = match acceptance {
        FileTransferAcceptance::Ready {
            transfer_id,
            resume_offset,
            operation_id,
        } => file_transfer_acceptance_v2::Result::Ready(FileTransferReadyV2 {
            transfer_id: transfer_id.to_bytes().to_vec(),
            resume_offset: *resume_offset,
            operation_id: operation_id.to_bytes().to_vec(),
        }),
        FileTransferAcceptance::AlreadyComplete { transfer_id } => {
            file_transfer_acceptance_v2::Result::AlreadyComplete(FileTransferAlreadyCompleteV2 {
                transfer_id: transfer_id.to_bytes().to_vec(),
            })
        }
    };

    encode_bounded(
        &FileTransferAcceptanceV2 {
            profile_version: FILE_TRANSFER_PROFILE_V2,
            result: Some(result),
        },
        MAX_FILE_TRANSFER_ACCEPTANCE_WIRE_BYTES,
    )
}

pub fn decode_file_transfer_acceptance(
    payload: &[u8],
) -> Result<FileTransferAcceptance, FileTransferWireError> {
    let wire: FileTransferAcceptanceV2 =
        decode_bounded(payload, MAX_FILE_TRANSFER_ACCEPTANCE_WIRE_BYTES)?;
    require_profile(wire.profile_version)?;

    match wire
        .result
        .ok_or(FileTransferWireError::MissingAcceptance)?
    {
        file_transfer_acceptance_v2::Result::Ready(ready) => Ok(FileTransferAcceptance::Ready {
            transfer_id: decode_transfer_id(ready.transfer_id)?,
            resume_offset: ready.resume_offset,
            operation_id: OperationId::from_bytes(copy_32(
                ready.operation_id,
                FileTransferWireError::InvalidOperationIdLength,
            )?),
        }),
        file_transfer_acceptance_v2::Result::AlreadyComplete(complete) => {
            Ok(FileTransferAcceptance::AlreadyComplete {
                transfer_id: decode_transfer_id(complete.transfer_id)?,
            })
        }
    }
}

pub fn encode_file_transfer_result(
    result: FileTransferResult,
) -> Result<Vec<u8>, FileTransferWireError> {
    let outcome = match result.outcome() {
        FileTransferTerminalOutcome::Completed => FileTransferTerminalOutcomeV2::Completed,
        FileTransferTerminalOutcome::Cancelled => FileTransferTerminalOutcomeV2::Cancelled,
        FileTransferTerminalOutcome::IntegrityFailed => {
            FileTransferTerminalOutcomeV2::IntegrityFailed
        }
        FileTransferTerminalOutcome::StorageFailed => FileTransferTerminalOutcomeV2::StorageFailed,
    };

    encode_bounded(
        &FileTransferResultV2 {
            profile_version: FILE_TRANSFER_PROFILE_V2,
            transfer_id: result.transfer_id().to_bytes().to_vec(),
            outcome: outcome as i32,
        },
        MAX_FILE_TRANSFER_RESULT_WIRE_BYTES,
    )
}

pub fn decode_file_transfer_result(
    payload: &[u8],
) -> Result<FileTransferResult, FileTransferWireError> {
    let wire: FileTransferResultV2 = decode_bounded(payload, MAX_FILE_TRANSFER_RESULT_WIRE_BYTES)?;
    require_profile(wire.profile_version)?;

    let outcome = match wire.outcome {
        value if value == FileTransferTerminalOutcomeV2::Completed as i32 => {
            FileTransferTerminalOutcome::Completed
        }
        value if value == FileTransferTerminalOutcomeV2::Cancelled as i32 => {
            FileTransferTerminalOutcome::Cancelled
        }
        value if value == FileTransferTerminalOutcomeV2::IntegrityFailed as i32 => {
            FileTransferTerminalOutcome::IntegrityFailed
        }
        value if value == FileTransferTerminalOutcomeV2::StorageFailed as i32 => {
            FileTransferTerminalOutcome::StorageFailed
        }
        value => return Err(FileTransferWireError::InvalidTerminalOutcome(value)),
    };

    Ok(FileTransferResult::new(
        decode_transfer_id(wire.transfer_id)?,
        outcome,
    ))
}

fn require_profile(profile: u32) -> Result<(), FileTransferWireError> {
    if profile == FILE_TRANSFER_PROFILE_V2 {
        Ok(())
    } else {
        Err(FileTransferWireError::InvalidProfile(profile))
    }
}

fn decode_transfer_id(bytes: Vec<u8>) -> Result<TransferId, FileTransferWireError> {
    copy_32(bytes, FileTransferWireError::InvalidTransferIdLength).map(TransferId::from_bytes)
}

fn encode_bounded(message: &impl Message, max: usize) -> Result<Vec<u8>, FileTransferWireError> {
    let encoded = message.encode_to_vec();
    if encoded.len() > max {
        return Err(FileTransferWireError::PayloadTooLarge {
            actual: encoded.len(),
            max,
        });
    }
    Ok(encoded)
}

fn decode_bounded<M: Message + Default>(
    payload: &[u8],
    max: usize,
) -> Result<M, FileTransferWireError> {
    if payload.len() > max {
        return Err(FileTransferWireError::PayloadTooLarge {
            actual: payload.len(),
            max,
        });
    }
    M::decode(payload).map_err(|_| FileTransferWireError::MalformedProtobuf)
}

fn copy_32(
    bytes: Vec<u8>,
    error: fn(usize) -> FileTransferWireError,
) -> Result<[u8; 32], FileTransferWireError> {
    let actual = bytes.len();
    bytes.try_into().map_err(|_| error(actual))
}
