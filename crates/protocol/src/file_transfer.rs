use core::fmt;

use crosslab_crypto::random_bytes;
use crosslab_policy::OperationId;

pub const FILE_TRANSFER_CAPABILITY_ID: &str = "files.transfer";
pub const FILE_TRANSFER_RESULT_EVENT_TYPE: &str = "files.transfer.result";
pub const FILE_TRANSFER_PROFILE_V2: u32 = 2;
pub const FILE_TRANSFER_CHECKPOINT_BYTES: u64 = 1_048_576;
pub const MAX_FILE_TRANSFER_DISPLAY_NAME_BYTES: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferIdGenerationError;

impl fmt::Display for TransferIdGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("secure transfer identifier generation failed")
    }
}

impl std::error::Error for TransferIdGenerationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransferId([u8; 32]);

impl TransferId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn generate() -> Result<Self, TransferIdGenerationError> {
        random_bytes::<32>()
            .map(Self)
            .map_err(|_| TransferIdGenerationError)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FileTransferDigest([u8; 32]);

impl FileTransferDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for FileTransferDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FileTransferDigest([REDACTED; 32 bytes])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferProfileError {
    InvalidDisplayName,
    TransferIdMismatch,
    InvalidResumeOffset,
}

impl fmt::Display for FileTransferProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidDisplayName => "file transfer display name is invalid",
            Self::TransferIdMismatch => "file transfer response belongs to a different transfer",
            Self::InvalidResumeOffset => "file transfer resume offset is invalid",
        })
    }
}

impl std::error::Error for FileTransferProfileError {}

#[derive(Clone, PartialEq, Eq)]
pub struct FileTransferOffer {
    transfer_id: TransferId,
    display_name: String,
    file_size: u64,
    digest: FileTransferDigest,
}

impl FileTransferOffer {
    pub fn new(
        transfer_id: TransferId,
        display_name: String,
        file_size: u64,
        digest: FileTransferDigest,
    ) -> Result<Self, FileTransferProfileError> {
        if !valid_display_name(&display_name) {
            return Err(FileTransferProfileError::InvalidDisplayName);
        }

        Ok(Self {
            transfer_id,
            display_name,
            file_size,
            digest,
        })
    }

    pub const fn transfer_id(&self) -> TransferId {
        self.transfer_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub const fn file_size(&self) -> u64 {
        self.file_size
    }

    pub const fn digest(&self) -> FileTransferDigest {
        self.digest
    }

    pub fn validate_resume_offset(
        &self,
        resume_offset: u64,
    ) -> Result<(), FileTransferProfileError> {
        if valid_resume_offset(resume_offset, self.file_size) {
            Ok(())
        } else {
            Err(FileTransferProfileError::InvalidResumeOffset)
        }
    }
}

impl fmt::Debug for FileTransferOffer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferOffer")
            .field("transfer_id", &self.transfer_id)
            .field("display_name_len", &self.display_name.len())
            .field("file_size", &self.file_size)
            .field("digest", &self.digest)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum FileTransferAcceptance {
    Ready {
        transfer_id: TransferId,
        resume_offset: u64,
        operation_id: OperationId,
    },
    AlreadyComplete {
        transfer_id: TransferId,
    },
}

impl FileTransferAcceptance {
    pub const fn transfer_id(&self) -> TransferId {
        match self {
            Self::Ready { transfer_id, .. } | Self::AlreadyComplete { transfer_id } => *transfer_id,
        }
    }

    pub fn validate_for_offer(
        &self,
        offer: &FileTransferOffer,
    ) -> Result<(), FileTransferProfileError> {
        if self.transfer_id() != offer.transfer_id() {
            return Err(FileTransferProfileError::TransferIdMismatch);
        }
        if let Self::Ready { resume_offset, .. } = self {
            offer.validate_resume_offset(*resume_offset)?;
        }
        Ok(())
    }
}

impl fmt::Debug for FileTransferAcceptance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready {
                transfer_id,
                resume_offset,
                ..
            } => formatter
                .debug_struct("Ready")
                .field("transfer_id", transfer_id)
                .field("resume_offset", resume_offset)
                .field("operation_id", &"[REDACTED; 32 bytes]")
                .finish(),
            Self::AlreadyComplete { transfer_id } => formatter
                .debug_struct("AlreadyComplete")
                .field("transfer_id", transfer_id)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferTerminalOutcome {
    Completed,
    Cancelled,
    IntegrityFailed,
    StorageFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTransferResult {
    transfer_id: TransferId,
    outcome: FileTransferTerminalOutcome,
}

impl FileTransferResult {
    pub const fn new(transfer_id: TransferId, outcome: FileTransferTerminalOutcome) -> Self {
        Self {
            transfer_id,
            outcome,
        }
    }

    pub const fn transfer_id(self) -> TransferId {
        self.transfer_id
    }

    pub const fn outcome(self) -> FileTransferTerminalOutcome {
        self.outcome
    }
}

pub const fn valid_resume_offset(resume_offset: u64, file_size: u64) -> bool {
    resume_offset <= file_size
        && (resume_offset == file_size
            || resume_offset.is_multiple_of(FILE_TRANSFER_CHECKPOINT_BYTES))
}

fn valid_display_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_FILE_TRANSFER_DISPLAY_NAME_BYTES
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains('\0')
}
