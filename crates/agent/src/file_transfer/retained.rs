use core::fmt;
use std::collections::BTreeSet;

use crosslab_crypto::blake3_256;
use crosslab_identity::DeviceId;
use crosslab_protocol::{
    FileTransferDigest, FileTransferOffer, MAX_FILE_TRANSFER_DISPLAY_NAME_BYTES, TransferId,
};

pub const MAX_RETAINED_FILE_TRANSFERS: usize = 64;
pub const MAX_FILE_TRANSFER_LOCAL_LOCATOR_BYTES: usize = 8 * 1024;
pub const MAX_FILE_TRANSFER_STATE_BYTES: usize = 1024 * 1024;

const STATE_SCHEMA_VERSION: u16 = 1;
const STATE_HEADER_BYTES: usize = 2 + 2 + 4 + 32;
const PARTIAL_TAG: u8 = 1;
const COMPLETED_TAG: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferStateError {
    InvalidLocator,
    InvalidCheckpoint,
    CheckpointRegression,
    PartialShorterThanCheckpoint,
    IdentityMismatch,
    AlreadyComplete,
    TooManyEntries,
    SnapshotTooLarge,
    UnsupportedSchema,
    MalformedSnapshot,
    SnapshotDigestMismatch,
    DuplicateTransferId,
    InvalidOffer,
}

impl fmt::Display for FileTransferStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLocator => "file transfer local locator is invalid",
            Self::InvalidCheckpoint => "file transfer durable checkpoint is invalid",
            Self::CheckpointRegression => "file transfer durable checkpoint regressed",
            Self::PartialShorterThanCheckpoint => {
                "partial file is shorter than its durable checkpoint"
            }
            Self::IdentityMismatch => "retained file transfer identity does not match the offer",
            Self::AlreadyComplete => "retained file transfer is already complete",
            Self::TooManyEntries => "too many retained file transfers",
            Self::SnapshotTooLarge => "retained file transfer snapshot exceeds its size limit",
            Self::UnsupportedSchema => "retained file transfer snapshot schema is unsupported",
            Self::MalformedSnapshot => "retained file transfer snapshot is malformed",
            Self::SnapshotDigestMismatch => "retained file transfer snapshot digest does not match",
            Self::DuplicateTransferId => "retained file transfer snapshot repeats a transfer id",
            Self::InvalidOffer => "retained file transfer offer identity is invalid",
        })
    }
}

impl std::error::Error for FileTransferStateError {}

#[derive(Clone, PartialEq, Eq)]
pub struct FileTransferLocalLocator(Vec<u8>);

impl FileTransferLocalLocator {
    pub fn new(bytes: Vec<u8>) -> Result<Self, FileTransferStateError> {
        if bytes.is_empty() || bytes.len() > MAX_FILE_TRANSFER_LOCAL_LOCATOR_BYTES {
            return Err(FileTransferStateError::InvalidLocator);
        }
        Ok(Self(bytes))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl fmt::Debug for FileTransferLocalLocator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "[REDACTED; {} bytes]", self.0.len())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FileTransferIdentity {
    source_device_id: DeviceId,
    offer: FileTransferOffer,
}

impl FileTransferIdentity {
    pub const fn new(source_device_id: DeviceId, offer: FileTransferOffer) -> Self {
        Self {
            source_device_id,
            offer,
        }
    }

    pub const fn source_device_id(&self) -> DeviceId {
        self.source_device_id
    }

    pub const fn offer(&self) -> &FileTransferOffer {
        &self.offer
    }

    pub const fn transfer_id(&self) -> TransferId {
        self.offer.transfer_id()
    }

    fn matches(&self, source_device_id: DeviceId, offer: &FileTransferOffer) -> bool {
        self.source_device_id == source_device_id && self.offer == *offer
    }
}

impl fmt::Debug for FileTransferIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferIdentity")
            .field("source_device_id", &self.source_device_id)
            .field("offer", &self.offer)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferRecoveryAction {
    Keep,
    TruncateTo(u64),
}

#[derive(Clone, PartialEq, Eq)]
pub struct FileTransferPartialState {
    identity: FileTransferIdentity,
    durable_offset: u64,
    locator: FileTransferLocalLocator,
    updated_at_unix_secs: u64,
}

impl FileTransferPartialState {
    pub fn new(
        identity: FileTransferIdentity,
        durable_offset: u64,
        locator: FileTransferLocalLocator,
        updated_at_unix_secs: u64,
    ) -> Result<Self, FileTransferStateError> {
        identity
            .offer()
            .validate_resume_offset(durable_offset)
            .map_err(|_| FileTransferStateError::InvalidCheckpoint)?;
        Ok(Self {
            identity,
            durable_offset,
            locator,
            updated_at_unix_secs,
        })
    }

    pub const fn identity(&self) -> &FileTransferIdentity {
        &self.identity
    }

    pub const fn durable_offset(&self) -> u64 {
        self.durable_offset
    }

    pub const fn locator(&self) -> &FileTransferLocalLocator {
        &self.locator
    }

    pub const fn updated_at_unix_secs(&self) -> u64 {
        self.updated_at_unix_secs
    }

    pub fn commit_checkpoint(
        &mut self,
        durable_offset: u64,
        updated_at_unix_secs: u64,
    ) -> Result<bool, FileTransferStateError> {
        if durable_offset < self.durable_offset {
            return Err(FileTransferStateError::CheckpointRegression);
        }
        self.identity
            .offer()
            .validate_resume_offset(durable_offset)
            .map_err(|_| FileTransferStateError::InvalidCheckpoint)?;
        let changed = durable_offset != self.durable_offset;
        self.durable_offset = durable_offset;
        self.updated_at_unix_secs = updated_at_unix_secs;
        Ok(changed)
    }

    pub fn recovery_action(
        &self,
        partial_file_len: u64,
    ) -> Result<FileTransferRecoveryAction, FileTransferStateError> {
        if partial_file_len < self.durable_offset {
            return Err(FileTransferStateError::PartialShorterThanCheckpoint);
        }
        if partial_file_len > self.durable_offset {
            return Ok(FileTransferRecoveryAction::TruncateTo(self.durable_offset));
        }
        Ok(FileTransferRecoveryAction::Keep)
    }
}

impl fmt::Debug for FileTransferPartialState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferPartialState")
            .field("identity", &self.identity)
            .field("durable_offset", &self.durable_offset)
            .field("locator", &self.locator)
            .field("updated_at_unix_secs", &self.updated_at_unix_secs)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FileTransferCompletionTombstone {
    identity: FileTransferIdentity,
    updated_at_unix_secs: u64,
}

impl FileTransferCompletionTombstone {
    pub const fn new(identity: FileTransferIdentity, updated_at_unix_secs: u64) -> Self {
        Self {
            identity,
            updated_at_unix_secs,
        }
    }

    pub const fn identity(&self) -> &FileTransferIdentity {
        &self.identity
    }

    pub const fn updated_at_unix_secs(&self) -> u64 {
        self.updated_at_unix_secs
    }
}

impl fmt::Debug for FileTransferCompletionTombstone {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferCompletionTombstone")
            .field("identity", &self.identity)
            .field("updated_at_unix_secs", &self.updated_at_unix_secs)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum FileTransferRetainedState {
    Partial(FileTransferPartialState),
    Completed(FileTransferCompletionTombstone),
}

impl FileTransferRetainedState {
    pub const fn identity(&self) -> &FileTransferIdentity {
        match self {
            Self::Partial(partial) => partial.identity(),
            Self::Completed(completed) => completed.identity(),
        }
    }

    pub const fn updated_at_unix_secs(&self) -> u64 {
        match self {
            Self::Partial(partial) => partial.updated_at_unix_secs(),
            Self::Completed(completed) => completed.updated_at_unix_secs(),
        }
    }
}

impl fmt::Debug for FileTransferRetainedState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Partial(partial) => formatter.debug_tuple("Partial").field(partial).finish(),
            Self::Completed(completed) => {
                formatter.debug_tuple("Completed").field(completed).finish()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferStateMatch<'a> {
    Partial(&'a FileTransferPartialState),
    AlreadyComplete(&'a FileTransferCompletionTombstone),
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct FileTransferStateSnapshot {
    entries: Vec<FileTransferRetainedState>,
}

impl FileTransferStateSnapshot {
    pub fn new(entries: Vec<FileTransferRetainedState>) -> Result<Self, FileTransferStateError> {
        if entries.len() > MAX_RETAINED_FILE_TRANSFERS {
            return Err(FileTransferStateError::TooManyEntries);
        }
        ensure_unique_transfer_ids(&entries)?;
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[FileTransferRetainedState] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn find(
        &self,
        source_device_id: DeviceId,
        offer: &FileTransferOffer,
    ) -> Result<Option<FileTransferStateMatch<'_>>, FileTransferStateError> {
        let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.identity().transfer_id() == offer.transfer_id())
        else {
            return Ok(None);
        };
        if !entry.identity().matches(source_device_id, offer) {
            return Err(FileTransferStateError::IdentityMismatch);
        }
        Ok(Some(match entry {
            FileTransferRetainedState::Partial(partial) => FileTransferStateMatch::Partial(partial)
            FileTransferRetainedState::Completed(completed) => {
                FileTransferStateMatch::AlreadyComplete(completed)
            }
        }))
    }

    pub fn upsert_partial(
        &mut self,
        partial: FileTransferPartialState,
    ) -> Result<(), FileTransferStateError> {
        let transfer_id = partial.identity().transfer_id();
        if let Some(position) = self
            .entries
            .iter()
            .position(|entry| entry.identity().transfer_id() == transfer_id)
        {
            if entry_identity(&self.entries[position]) != partial.identity() {
                return Err(FileTransferStateError::IdentityMismatch);
            }
            if matches!(
                self.entries[position],
                FileTransferRetainedState::Completed(_)
            ) {
                return Err(FileTransferStateError::AlreadyComplete);
            }
            self.entries.remove(position);
        } else if self.entries.len() >= MAX_RETAINED_FILE_TRANSFERS {
            return Err(FileTransferStateError::TooManyEntries);
        }
        self.entries
            .push(FileTransferRetainedState::Partial(partial));
        Ok(())
    }

    pub fn mark_completed(
        &mut self,
        completed: FileTransferCompletionTombstone,
    ) -> Result<(), FileTransferStateError> {
        let transfer_id = completed.identity().transfer_id();
        if let Some(position) = self
            .entries
            .iter()
            .position(|entry| entry.identity().transfer_id() == transfer_id)
        {
            if entry_identity(&self.entries[position]) != completed.identity() {
                return Err(FileTransferStateError::IdentityMismatch);
            }
            self.entries.remove(position);
        } else if self.entries.len() >= MAX_RETAINED_FILE_TRANSFERS {
            return Err(FileTransferStateError::TooManyEntries);
        }
        self.entries
            .push(FileTransferRetainedState::Completed(completed));
        Ok(())
    }

    pub fn remove(&mut self, transfer_id: TransferId) -> bool {
        let Some(position) = self
            .entries
            .iter()
            .position(|entry| entry.identity().transfer_id() == transfer_id)
        else {
            return false;
        };
        self.entries.remove(position);
        true
    }

    pub fn prune_expired(&mut self, now_unix_secs: u64, max_age_secs: u64) -> usize {
        let before = self.entries.len();
        self.entries.retain(|entry| {
            now_unix_secs < entry.updated_at_unix_secs()
                || now_unix_secs.saturating_sub(entry.updated_at_unix_secs()) <= max_age_secs
        });
        before - self.entries.len()
    }

    pub fn encode(&self) -> Result<Vec<u8>, FileTransferStateError> {
        if self.entries.len() > MAX_RETAINED_FILE_TRANSFERS {
            return Err(FileTransferStateError::TooManyEntries);
        }
        ensure_unique_transfer_ids(&self.entries)?;

        let mut payload = Vec::new();
        for entry in &self.entries {
            encode_entry(&mut payload, entry)?;
            if payload.len() + STATE_HEADER_BYTES > MAX_FILE_TRANSFER_STATE_BYTES {
                return Err(FileTransferStateError::SnapshotTooLarge);
            }
        }

        let payload_len =
            u32::try_from(payload.len()).map_err(|_| FileTransferStateError::SnapshotTooLarge)?;
        let entry_count = u16::try_from(self.entries.len())
            .map_err(|_| FileTransferStateError::TooManyEntries)?;
        let digest = blake3_256(&payload);
        let mut encoded = Vec::with_capacity(STATE_HEADER_BYTES + payload.len());
        encoded.extend_from_slice(&STATE_SCHEMA_VERSION.to_be_bytes());
        encoded.extend_from_slice(&entry_count.to_be_bytes());
        encoded.extend_from_slice(&payload_len.to_be_bytes());
        encoded.extend_from_slice(&digest);
        encoded.extend_from_slice(&payload);
        Ok(encoded)
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, FileTransferStateError> {
        if encoded.len() > MAX_FILE_TRANSFER_STATE_BYTES {
            return Err(FileTransferStateError::SnapshotTooLarge);
        }
        if encoded.len() < STATE_HEADER_BYTES {
            return Err(FileTransferStateError::MalformedSnapshot);
        }

        let schema = u16::from_be_bytes(copy_array(&encoded[0..2]));
        if schema != STATE_SCHEMA_VERSION {
            return Err(FileTransferStateError::UnsupportedSchema);
        }
        let entry_count = usize::from(u16::from_be_bytes(copy_array(&encoded[2..4])));
        if entry_count > MAX_RETAINED_FILE_TRANSFERS {
            return Err(FileTransferStateError::TooManyEntries);
        }
        let payload_len = usize::try_from(u32::from_be_bytes(copy_array(&encoded[4..8])))
            .map_err(|_| FileTransferStateError::MalformedSnapshot)?;
        let expected_len = STATE_HEADER_BYTES
            .checked_add(payload_len)
            .ok_or(FileTransferStateError::MalformedSnapshot)?;
        if encoded.len() != expected_len {
            return Err(FileTransferStateError::MalformedSnapshot);
        }
        let expected_digest: [u8; 32] = copy_array(&encoded[8..40]);
        let payload = &encoded[STATE_HEADER_BYTES..];
        if blake3_256(payload) != expected_digest {
            return Err(FileTransferStateError::SnapshotDigestMismatch);
        }

        let mut reader = Reader::new(payload);
        let mut entries = Vec::with_capacity(entry_count);
        let mut transfer_ids = BTreeSet::new();
        for _ in 0..entry_count {
            let entry = decode_entry(&mut reader)?;
            if !transfer_ids.insert(entry.identity().transfer_id()) {
                return Err(FileTransferStateError::DuplicateTransferId);
            }
            entries.push(entry);
        }
        reader.finish()?;
        Ok(Self { entries })
    }
}

impl fmt::Debug for FileTransferStateSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let partial = self
            .entries
            .iter()
            .filter(|entry| matches!(entry, FileTransferRetainedState::Partial(_)))
            .count();
        formatter
            .debug_struct("FileTransferStateSnapshot")
            .field("entries", &self.entries.len())
            .field("partial", &partial)
            .field("completed", &(self.entries.len() - partial))
            .finish()
    }
}

fn entry_identity(entry: &FileTransferRetainedState) -> &FileTransferIdentity {
    entry.identity()
}

fn ensure_unique_transfer_ids(
    entries: &[FileTransferRetainedState],
) -> Result<(), FileTransferStateError> {
    let mut transfer_ids = BTreeSet::new();
    for entry in entries {
        if !transfer_ids.insert(entry.identity().transfer_id()) {
            return Err(FileTransferStateError::DuplicateTransferId);
        }
    }
    Ok(())
}

fn encode_entry(
    encoded: &mut Vec<u8>,
    entry: &FileTransferRetainedState,
) -> Result<(), FileTransferStateError> {
    let (tag, identity, updated_at) = match entry {
        FileTransferRetainedState::Partial(partial) => (
            PARTIAL_TAG,
            partial.identity(),
            partial.updated_at_unix_secs(),
        )
        FileTransferRetainedState::Completed(completed) => (
            COMPLETED_TAG,
            completed.identity(),
            completed.updated_at_unix_secs(),
        ),
    };
    encoded.push(tag);
    encoded.extend_from_slice(&identity.transfer_id().to_bytes());
    encoded.extend_from_slice(identity.source_device_id().as_bytes());
    push_u16_bytes(
        encoded,
        identity.offer().display_name().as_bytes(),
        MAX_FILE_TRANSFER_DISPLAY_NAME_BYTES,
    )?;
    encoded.extend_from_slice(&identity.offer().file_size().to_be_bytes());
    encoded.extend_from_slice(&identity.offer().digest().to_bytes());
    encoded.extend_from_slice(&updated_at.to_be_bytes());

    if let FileTransferRetainedState::Partial(partial) = entry {
        encoded.extend_from_slice(&partial.durable_offset().to_be_bytes());
        push_u16_bytes(
            encoded,
            partial.locator().bytes(),
            MAX_FILE_TRANSFER_LOCAL_LOCATOR_BYTES,
        )?;
    }
    Ok(())
}

fn decode_entry(
    reader: &mut Reader<'_>,
) -> Result<FileTransferRetainedState, FileTransferStateError> {
    let tag = reader.u8()?;
    let transfer_id = TransferId::from_bytes(reader.array()?);
    let source_device_id = DeviceId::from_bytes(reader.array()?);
    let display_name =
        core::str::from_utf8(reader.bytes_u16(MAX_FILE_TRANSFER_DISPLAY_NAME_BYTES)?)
            .map_err(|_| FileTransferStateError::InvalidOffer)?
            .to_owned();
    let file_size = reader.u64()?;
    let digest = FileTransferDigest::from_bytes(reader.array()?);
    let updated_at = reader.u64()?;
    let offer = FileTransferOffer::new(transfer_id, display_name, file_size, digest)
        .map_err(|_| FileTransferStateError::InvalidOffer)?;
    let identity = FileTransferIdentity::new(source_device_id, offer);

    match tag {
        PARTIAL_TAG => {
            let durable_offset = reader.u64()?;
            let locator = FileTransferLocalLocator::new(
                reader
                    .bytes_u16(MAX_FILE_TRANSFER_LOCAL_LOCATOR_BYTES)?
                    .to_vec(),
            )?;
            Ok(FileTransferRetainedState::Partial(
                FileTransferPartialState::new(identity, durable_offset, locator, updated_at)?,
            ))
        }
        COMPLETED_TAG => Ok(FileTransferRetainedState::Completed(
            FileTransferCompletionTombstone::new(identity, updated_at),
        )),
        _ => Err(FileTransferStateError::MalformedSnapshot),
    }
}

fn push_u16_bytes(
    encoded: &mut Vec<u8>,
    bytes: &[u8],
    max_len: usize,
) -> Result<(), FileTransferStateError> {
    if bytes.len() > max_len {
        return Err(FileTransferStateError::SnapshotTooLarge);
    }
    let len = u16::try_from(bytes.len()).map_err(|_| FileTransferStateError::SnapshotTooLarge)?;
    encoded.extend_from_slice(&len.to_be_bytes());
    encoded.extend_from_slice(bytes);
    Ok(())
}

fn copy_array<const N: usize>(bytes: &[u8]) -> [u8; N] {
    bytes
        .try_into()
        .expect("validated retained-state slice length")
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn u8(&mut self) -> Result<u8, FileTransferStateError> {
        Ok(self.take(1)?[0])
    }

    fn u64(&mut self) -> Result<u64, FileTransferStateError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], FileTransferStateError> {
        Ok(copy_array(self.take(N)?))
    }

    fn bytes_u16(&mut self, max_len: usize) -> Result<&'a [u8], FileTransferStateError> {
        let len = usize::from(u16::from_be_bytes(self.array()?));
        if len > max_len {
            return Err(FileTransferStateError::MalformedSnapshot);
        }
        self.take(len)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], FileTransferStateError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(FileTransferStateError::MalformedSnapshot)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(FileTransferStateError::MalformedSnapshot)?;
        self.offset = end;
        Ok(value)
    }

    fn finish(self) -> Result<(), FileTransferStateError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(FileTransferStateError::MalformedSnapshot)
        }
    }
}

#[cfg(test)]
mod tests {
    use crosslab_protocol::FILE_TRANSFER_CHECKPOINT_BYTES;

    use super::*;

    fn offer(transfer: u8, digest: u8) -> FileTransferOffer {
        FileTransferOffer::new(
            TransferId::from_bytes([transfer; 32]),
            "resume.bin".into(),
            FILE_TRANSFER_CHECKPOINT_BYTES + 17,
            FileTransferDigest::from_bytes([digest; 32]),
        )
        .unwrap()
    }

    fn identity(transfer: u8, source: u8, digest: u8) -> FileTransferIdentity {
        FileTransferIdentity::new(DeviceId::from_bytes([source; 32]), offer(transfer, digest))
    }

    fn locator() -> FileTransferLocalLocator {
        FileTransferLocalLocator::new(b"/private/platform/partial".to_vec()).unwrap()
    }

    #[test]
    fn checkpoint_recovery_uses_only_durable_contiguous_bytes() {
        let checkpoint = FILE_TRANSFER_CHECKPOINT_BYTES;
        let mut partial =
            FileTransferPartialState::new(identity(1, 2, 3), checkpoint, locator(), 10).unwrap();

        assert_eq!(
            partial.recovery_action(checkpoint),
            Ok(FileTransferRecoveryAction::Keep)
        );
        assert_eq!(
            partial.recovery_action(checkpoint + 123),
            Ok(FileTransferRecoveryAction::TruncateTo(checkpoint))
        );
        assert_eq!(
            partial.recovery_action(checkpoint - 1),
            Err(FileTransferStateError::PartialShorterThanCheckpoint)
        );
        assert_eq!(
            partial.commit_checkpoint(1, 11),
            Err(FileTransferStateError::CheckpointRegression)
        );
        assert_eq!(
            partial.commit_checkpoint(checkpoint + 1, 11),
            Err(FileTransferStateError::InvalidCheckpoint)
        );
        assert_eq!(
            partial
                .commit_checkpoint(FILE_TRANSFER_CHECKPOINT_BYTES + 17, 12)
                .unwrap(),
            true
        );
    }

    #[test]
    fn retained_identity_rejects_changed_source_or_offer() {
        let partial = FileTransferPartialState::new(identity(4, 5, 6), 0, locator(), 10).unwrap();
        let mut state = FileTransferStateSnapshot::default();
        state.upsert_partial(partial).unwrap();

        assert!(matches!(
            state.find(DeviceId::from_bytes([5; 32]), &offer(4, 6)),
            Ok(Some(FileTransferStateMatch::Partial(_)))
        ));
        assert_eq!(
            state.find(DeviceId::from_bytes([7; 32]), &offer(4, 6)),
            Err(FileTransferStateError::IdentityMismatch)
        );
        assert_eq!(
            state.find(DeviceId::from_bytes([5; 32]), &offer(4, 8)),
            Err(FileTransferStateError::IdentityMismatch)
        );
    }

    #[test]
    fn completion_tombstone_is_idempotent_and_cannot_downgrade() {
        let id = identity(9, 10, 11);
        let partial = FileTransferPartialState::new(id.clone(), 0, locator(), 20).unwrap();
        let completed = FileTransferCompletionTombstone::new(id.clone(), 21);
        let mut state = FileTransferStateSnapshot::default();
        state.upsert_partial(partial.clone()).unwrap();
        state.mark_completed(completed.clone()).unwrap();
        state.mark_completed(completed).unwrap();

        assert!(matches!(
            state.find(id.source_device_id(), id.offer()),
            Ok(Some(FileTransferStateMatch::AlreadyComplete(_)))
        ));
        assert_eq!(
            state.upsert_partial(partial),
            Err(FileTransferStateError::AlreadyComplete)
        );
    }

    #[test]
    fn state_snapshot_round_trips_and_redacts_local_locator() {
        let partial =
            FileTransferPartialState::new(identity(12, 13, 14), 0, locator(), 100).unwrap();
        let completed = FileTransferCompletionTombstone::new(identity(15, 16, 17), 101);
        let state = FileTransferStateSnapshot::new(vec![
            FileTransferRetainedState::Partial(partial),
            FileTransferRetainedState::Completed(completed),
        ])
        .unwrap();

        let encoded = state.encode().unwrap();
        let decoded = FileTransferStateSnapshot::decode(&encoded).unwrap();
        assert_eq!(decoded, state);

        let rendered = format!("{:?}", decoded.entries()[0]);
        assert!(!rendered.contains("/private/platform/partial"));
        assert!(!rendered.contains("resume.bin"));
        assert!(rendered.contains("[REDACTED; 25 bytes]"));

        let mut corrupted = encoded;
        *corrupted.last_mut().unwrap() ^= 0xff;
        assert_eq!(
            FileTransferStateSnapshot::decode(&corrupted),
            Err(FileTransferStateError::SnapshotDigestMismatch)
        );
    }

    #[test]
    fn retained_state_is_count_bounded_and_prunable() {
        let mut state = FileTransferStateSnapshot::default();
        for index in 0..MAX_RETAINED_FILE_TRANSFERS {
            let transfer = u8::try_from(index + 1).unwrap();
            state
                .upsert_partial(
                    FileTransferPartialState::new(
                        identity(transfer, 0x55, transfer),
                        0,
                        locator(),
                        index as u64,
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        assert_eq!(state.len(), MAX_RETAINED_FILE_TRANSFERS);
        let extra =
            FileTransferPartialState::new(identity(65, 0x55, 65), 0, locator(), 65).unwrap();
        assert_eq!(
            state.upsert_partial(extra.clone()),
            Err(FileTransferStateError::TooManyEntries)
        );

        let removed = state.prune_expired(100, 50);
        assert!(removed > 0);
        state.upsert_partial(extra).unwrap();
        assert!(state.len() <= MAX_RETAINED_FILE_TRANSFERS);
    }

    #[test]
    fn snapshot_rejects_duplicate_transfer_ids_and_oversized_locator() {
        let partial = FileTransferPartialState::new(identity(22, 23, 24), 0, locator(), 1).unwrap();
        assert_eq!(
            FileTransferStateSnapshot::new(vec![
                FileTransferRetainedState::Partial(partial.clone()),
                FileTransferRetainedState::Partial(partial),
            ]),
            Err(FileTransferStateError::DuplicateTransferId)
        );
        assert_eq!(
            FileTransferLocalLocator::new(vec![0; MAX_FILE_TRANSFER_LOCAL_LOCATOR_BYTES + 1]),
            Err(FileTransferStateError::InvalidLocator)
        );
    }
}
