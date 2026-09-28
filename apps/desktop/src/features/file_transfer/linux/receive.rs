use core::fmt;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read as _, Seek as _, SeekFrom, Write as _},
    os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
};

use crosslab_agent::{
    FileTransferCompletionTombstone, FileTransferIdentity, FileTransferIntegrityError,
    FileTransferPartialState, FileTransferRecoveryAction, FileTransferStateError,
    FileTransferStateMatch, FileTransferStateSnapshot, FileTransferVerifier,
};
use crosslab_identity::DeviceId;
use crosslab_protocol::{FILE_TRANSFER_CHECKPOINT_BYTES, FileTransferOffer};

use super::{
    FILE_TRANSFER_IO_CHUNK_BYTES, LinuxFileTransferError, LinuxFileTransferLocator,
    LinuxFileTransferStateStore,
};

#[derive(Debug)]
pub enum LinuxFileTransferRecovery {
    Ready(LinuxFileTransferReceiver),
    AlreadyComplete,
}

pub struct LinuxFileTransferReceiver {
    file: File,
    locator: LinuxFileTransferLocator,
    partial: FileTransferPartialState,
    store: LinuxFileTransferStateStore,
    offset: u64,
}

impl LinuxFileTransferReceiver {
    pub fn create(
        store: LinuxFileTransferStateStore,
        source_device_id: DeviceId,
        offer: FileTransferOffer,
        final_path: PathBuf,
        now_unix_secs: u64,
    ) -> Result<Self, LinuxFileTransferError> {
        let mut snapshot = store.load()?;
        match snapshot.find(source_device_id, &offer)? {
            Some(FileTransferStateMatch::AlreadyComplete(_)) => {
                return Err(LinuxFileTransferError::AlreadyComplete);
            }
            Some(FileTransferStateMatch::Partial(_)) => {
                return Err(LinuxFileTransferError::RetainedStateExists);
            }
            None => {}
        }

        let locator =
            LinuxFileTransferLocator::for_destination(final_path, offer.transfer_id())?;
        if path_exists(locator.final_path())? {
            return Err(LinuxFileTransferError::DestinationExists);
        }

        let identity = FileTransferIdentity::new(source_device_id, offer);
        let partial =
            FileTransferPartialState::new(identity, 0, locator.encode()?, now_unix_secs)?;
        snapshot.upsert_partial(partial.clone())?;

        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(locator.partial_path())
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(LinuxFileTransferError::PartialExists);
            }
            Err(error) => return Err(error.into()),
        };

        let prepare_result = file
            .sync_all()
            .map_err(LinuxFileTransferError::from)
            .and_then(|()| sync_parent(locator.partial_path()));
        if let Err(error) = prepare_result {
            drop(file);
            cleanup_uncommitted_partial(locator.partial_path());
            return Err(error);
        }
        if let Err(error) = store.commit(&snapshot) {
            drop(file);
            cleanup_uncommitted_partial(locator.partial_path());
            return Err(error);
        }

        Ok(Self {
            file,
            locator,
            partial,
            store,
            offset: 0,
        })
    }

    pub fn recover(
        store: LinuxFileTransferStateStore,
        source_device_id: DeviceId,
        offer: FileTransferOffer,
        now_unix_secs: u64,
    ) -> Result<LinuxFileTransferRecovery, LinuxFileTransferError> {
        let snapshot = store.load()?;
        let partial = match snapshot.find(source_device_id, &offer)? {
            Some(FileTransferStateMatch::AlreadyComplete(_)) => {
                return Ok(LinuxFileTransferRecovery::AlreadyComplete);
            }
            Some(FileTransferStateMatch::Partial(partial)) => partial.clone(),
            None => return Err(LinuxFileTransferError::RetainedStateMissing),
        };
        let locator =
            LinuxFileTransferLocator::decode(partial.locator(), offer.transfer_id())?;
        let partial_exists = path_exists(locator.partial_path())?;
        let final_exists = path_exists(locator.final_path())?;

        if final_exists {
            if partial_exists {
                let mut file = open_existing_partial(locator.partial_path())?;
                if !same_regular_file(&file, locator.final_path())? {
                    return Err(LinuxFileTransferError::DestinationExists);
                }
                verify_file(&mut file, &offer)?;
                fs::remove_file(locator.partial_path())?;
            } else {
                let mut file = open_existing_final(locator.final_path())?;
                verify_file(&mut file, &offer)?;
            }
            sync_parent(locator.final_path())?;

            let completed =
                FileTransferCompletionTombstone::new(partial.identity().clone(), now_unix_secs);
            let mut next = snapshot.clone();
            next.mark_completed(completed)?;
            store.commit(&next)?;
            return Ok(LinuxFileTransferRecovery::AlreadyComplete);
        }
        if !partial_exists {
            return Err(LinuxFileTransferError::InvalidPartial);
        }

        let mut file = open_existing_partial(locator.partial_path())?;
        match partial.recovery_action(file.metadata()?.len())? {
            FileTransferRecoveryAction::Keep => {}
            FileTransferRecoveryAction::TruncateTo(offset) => {
                file.set_len(offset)?;
                file.sync_all()?;
            }
        }
        file.seek(SeekFrom::Start(partial.durable_offset()))?;

        Ok(LinuxFileTransferRecovery::Ready(Self {
            file,
            locator,
            offset: partial.durable_offset(),
            partial,
            store,
        }))
    }

    pub const fn offset(&self) -> u64 {
        self.offset
    }

    pub const fn durable_offset(&self) -> u64 {
        self.partial.durable_offset()
    }

    pub fn write_chunk(
        &mut self,
        bytes: &[u8],
        now_unix_secs: u64,
    ) -> Result<(), LinuxFileTransferError> {
        if bytes.len() > FILE_TRANSFER_IO_CHUNK_BYTES {
            return Err(LinuxFileTransferError::ChunkTooLarge);
        }
        let len =
            u64::try_from(bytes.len()).map_err(|_| FileTransferIntegrityError::SizeOverflow)?;
        let next = self
            .offset
            .checked_add(len)
            .ok_or(FileTransferIntegrityError::SizeOverflow)?;
        if next > self.partial.identity().offer().file_size() {
            return Err(FileTransferIntegrityError::SizeMismatch.into());
        }

        self.file.write_all(bytes)?;
        self.offset = next;
        self.persist_checkpoint(now_unix_secs)
    }

    pub fn finish(mut self, now_unix_secs: u64) -> Result<(), LinuxFileTransferError> {
        if self.offset != self.partial.identity().offer().file_size() {
            return Err(FileTransferIntegrityError::SizeMismatch.into());
        }
        self.persist_checkpoint(now_unix_secs)?;

        let offer = self.partial.identity().offer().clone();
        if let Err(error) = verify_file(&mut self.file, &offer) {
            if matches!(&error, LinuxFileTransferError::Integrity(_)) {
                self.invalidate_failed_integrity()?;
            }
            return Err(error);
        }

        match fs::hard_link(self.locator.partial_path(), self.locator.final_path()) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(LinuxFileTransferError::DestinationExists);
            }
            Err(error) => return Err(error.into()),
        }
        sync_parent(self.locator.final_path())?;
        fs::remove_file(self.locator.partial_path())?;
        sync_parent(self.locator.final_path())?;

        let completed =
            FileTransferCompletionTombstone::new(self.partial.identity().clone(), now_unix_secs);
        let mut snapshot = self.load_partial_snapshot()?;
        snapshot.mark_completed(completed)?;
        self.store.commit(&snapshot)?;
        Ok(())
    }

    fn persist_checkpoint(&mut self, now_unix_secs: u64) -> Result<(), LinuxFileTransferError> {
        let durable = durable_offset(self.offset, self.partial.identity().offer().file_size());
        if durable <= self.partial.durable_offset() {
            return Ok(());
        }

        self.file.sync_all()?;
        let mut partial = self.partial.clone();
        partial.commit_checkpoint(durable, now_unix_secs)?;
        let mut snapshot = self.load_partial_snapshot()?;
        snapshot.upsert_partial(partial.clone())?;
        self.store.commit(&snapshot)?;
        self.partial = partial;
        Ok(())
    }

    fn load_partial_snapshot(&self) -> Result<FileTransferStateSnapshot, LinuxFileTransferError> {
        let snapshot = self.store.load()?;
        match snapshot.find(
            self.partial.identity().source_device_id(),
            self.partial.identity().offer(),
        )? {
            Some(FileTransferStateMatch::Partial(current))
                if current.durable_offset() == self.partial.durable_offset() =>
            {
                Ok(snapshot)
            }
            Some(FileTransferStateMatch::Partial(_)) => {
                Err(FileTransferStateError::CheckpointRegression.into())
            }
            Some(FileTransferStateMatch::AlreadyComplete(_)) => {
                Err(LinuxFileTransferError::AlreadyComplete)
            }
            None => Err(LinuxFileTransferError::RetainedStateMissing),
        }
    }

    fn invalidate_failed_integrity(&mut self) -> Result<(), LinuxFileTransferError> {
        self.file.set_len(0)?;
        self.file.sync_all()?;

        let mut snapshot = self.load_partial_snapshot()?;
        snapshot.remove(self.partial.identity().transfer_id());
        self.store.commit(&snapshot)?;

        let partial_path = self.locator.partial_path().to_path_buf();
        let parent = partial_path.parent().map(Path::to_path_buf);
        let _ = fs::remove_file(&partial_path);
        if let Some(parent) = parent {
            let _ = File::open(parent).and_then(|directory| directory.sync_all());
        }
        Ok(())
    }
}

impl fmt::Debug for LinuxFileTransferReceiver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxFileTransferReceiver")
            .field("offset", &self.offset)
            .field("durable_offset", &self.partial.durable_offset())
            .field("expected_size", &self.partial.identity().offer().file_size())
            .field("locator", &self.locator)
            .finish()
    }
}

fn durable_offset(offset: u64, file_size: u64) -> u64 {
    if offset == file_size {
        offset
    } else {
        offset - (offset % FILE_TRANSFER_CHECKPOINT_BYTES)
    }
}

fn path_exists(path: &Path) -> Result<bool, LinuxFileTransferError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn open_existing_partial(path: &Path) -> Result<File, LinuxFileTransferError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(LinuxFileTransferError::InvalidPartial);
    }

    let file = OpenOptions::new().read(true).write(true).open(path)?;
    let opened = file.metadata()?;
    if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
        return Err(LinuxFileTransferError::InvalidPartial);
    }
    Ok(file)
}

fn same_regular_file(file: &File, path: &Path) -> Result<bool, LinuxFileTransferError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Ok(false);
    }
    let opened = file.metadata()?;
    Ok(opened.dev() == metadata.dev() && opened.ino() == metadata.ino())
}

fn open_existing_final(path: &Path) -> Result<File, LinuxFileTransferError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(LinuxFileTransferError::DestinationExists);
    }
    let file = File::open(path)?;
    let opened = file.metadata()?;
    if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
        return Err(LinuxFileTransferError::DestinationExists);
    }
    Ok(file)
}

fn verify_file(
    file: &mut File,
    offer: &FileTransferOffer,
) -> Result<(), LinuxFileTransferError> {
    file.seek(SeekFrom::Start(0))?;
    let mut verifier = FileTransferVerifier::new(offer);
    let mut buffer = vec![0_u8; FILE_TRANSFER_IO_CHUNK_BYTES];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        verifier.update(&buffer[..read])?;
    }
    verifier.finish().map_err(Into::into)
}

fn sync_parent(path: &Path) -> Result<(), LinuxFileTransferError> {
    let parent = path.parent().ok_or(LinuxFileTransferError::InvalidPath)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn cleanup_uncommitted_partial(path: &Path) {
    if fs::remove_file(path).is_ok() {
        let _ = sync_parent(path);
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fmt::Write as _};

    use crosslab_crypto::{blake3_256, random_bytes};
    use crosslab_protocol::{FileTransferDigest, TransferId};

    use super::*;

    #[test]
    fn interrupted_receive_recovers_only_durable_checkpoint_and_publishes() {
        let root = test_root("resume");
        fs::create_dir_all(&root).unwrap();
        let state_path = root.join("state.bin");
        let final_path = root.join("received.bin");
        let source = DeviceId::from_bytes([0x61; 32]);
        let len = usize::try_from(FILE_TRANSFER_CHECKPOINT_BYTES).unwrap() + 17;
        let payload = vec![0x5a; len];
        let offer = test_offer(0x62, "received.bin", &payload);

        let mut receiver = LinuxFileTransferReceiver::create(
            LinuxFileTransferStateStore::for_test(state_path.clone()),
            source,
            offer.clone(),
            final_path.clone(),
            10,
        )
        .unwrap();
        let checkpoint = usize::try_from(FILE_TRANSFER_CHECKPOINT_BYTES).unwrap();
        write_chunks(&mut receiver, &payload[..checkpoint + 5], 11);
        assert_eq!(receiver.durable_offset(), FILE_TRANSFER_CHECKPOINT_BYTES);
        let partial_path = receiver.locator.partial_path().to_path_buf();
        drop(receiver);

        let LinuxFileTransferRecovery::Ready(mut receiver) = LinuxFileTransferReceiver::recover(
            LinuxFileTransferStateStore::for_test(state_path.clone()),
            source,
            offer.clone(),
            12,
        )
        .unwrap()
        else {
            panic!("expected resumable receive");
        };
        assert_eq!(receiver.offset(), FILE_TRANSFER_CHECKPOINT_BYTES);
        assert_eq!(
            fs::metadata(&partial_path).unwrap().len(),
            FILE_TRANSFER_CHECKPOINT_BYTES
        );

        write_chunks(&mut receiver, &payload[checkpoint..], 13);
        receiver.finish(14).unwrap();

        assert_eq!(fs::read(&final_path).unwrap(), payload);
        assert!(!partial_path.exists());
        let state = LinuxFileTransferStateStore::for_test(state_path)
            .load()
            .unwrap();
        assert!(matches!(
            state.find(source, &offer),
            Ok(Some(FileTransferStateMatch::AlreadyComplete(_)))
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn digest_mismatch_discards_untrusted_partial_state() {
        let root = test_root("integrity");
        fs::create_dir_all(&root).unwrap();
        let state_path = root.join("state.bin");
        let final_path = root.join("received.bin");
        let source = DeviceId::from_bytes([0x71; 32]);
        let offer = test_offer(0x72, "received.bin", b"abcdef");

        let mut receiver = LinuxFileTransferReceiver::create(
            LinuxFileTransferStateStore::for_test(state_path.clone()),
            source,
            offer.clone(),
            final_path.clone(),
            20,
        )
        .unwrap();
        let partial_path = receiver.locator.partial_path().to_path_buf();
        receiver.write_chunk(b"abcdeg", 21).unwrap();
        assert!(matches!(
            receiver.finish(22),
            Err(LinuxFileTransferError::Integrity(
                FileTransferIntegrityError::DigestMismatch
            ))
        ));

        assert!(!final_path.exists());
        assert!(!partial_path.exists());
        let state = LinuxFileTransferStateStore::for_test(state_path)
            .load()
            .unwrap();
        assert!(matches!(state.find(source, &offer), Ok(None)));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn publication_never_clobbers_existing_owner_file() {
        let root = test_root("collision");
        fs::create_dir_all(&root).unwrap();
        let state_path = root.join("state.bin");
        let final_path = root.join("received.bin");
        let source = DeviceId::from_bytes([0x81; 32]);
        let offer = test_offer(0x82, "received.bin", b"payload");

        let mut receiver = LinuxFileTransferReceiver::create(
            LinuxFileTransferStateStore::for_test(state_path),
            source,
            offer,
            final_path.clone(),
            30,
        )
        .unwrap();
        receiver.write_chunk(b"payload", 31).unwrap();
        fs::write(&final_path, b"owner-data").unwrap();

        assert!(matches!(
            receiver.finish(32),
            Err(LinuxFileTransferError::DestinationExists)
        ));
        assert_eq!(fs::read(&final_path).unwrap(), b"owner-data");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovery_fails_closed_when_partial_is_shorter_than_checkpoint() {
        let root = test_root("short");
        fs::create_dir_all(&root).unwrap();
        let state_path = root.join("state.bin");
        let final_path = root.join("received.bin");
        let source = DeviceId::from_bytes([0x91; 32]);
        let checkpoint = usize::try_from(FILE_TRANSFER_CHECKPOINT_BYTES).unwrap();
        let payload = vec![0x33; checkpoint + 1];
        let offer = test_offer(0x92, "received.bin", &payload);

        let mut receiver = LinuxFileTransferReceiver::create(
            LinuxFileTransferStateStore::for_test(state_path.clone()),
            source,
            offer.clone(),
            final_path,
            40,
        )
        .unwrap();
        write_chunks(&mut receiver, &payload[..checkpoint], 41);
        let partial_path = receiver.locator.partial_path().to_path_buf();
        drop(receiver);
        OpenOptions::new()
            .write(true)
            .open(&partial_path)
            .unwrap()
            .set_len(FILE_TRANSFER_CHECKPOINT_BYTES - 1)
            .unwrap();

        assert!(matches!(
            LinuxFileTransferReceiver::recover(
                LinuxFileTransferStateStore::for_test(state_path),
                source,
                offer,
                42
            ),
            Err(LinuxFileTransferError::State(
                crosslab_agent::FileTransferStateError::PartialShorterThanCheckpoint
            ))
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovery_completes_tombstone_after_published_link_crash() {
        let root = test_root("published");
        fs::create_dir_all(&root).unwrap();
        let state_path = root.join("state.bin");
        let final_path = root.join("received.bin");
        let source = DeviceId::from_bytes([0xa1; 32]);
        let payload = b"published";
        let offer = test_offer(0xa2, "received.bin", payload);

        let mut receiver = LinuxFileTransferReceiver::create(
            LinuxFileTransferStateStore::for_test(state_path.clone()),
            source,
            offer.clone(),
            final_path.clone(),
            50,
        )
        .unwrap();
        receiver.write_chunk(payload, 51).unwrap();
        let partial_path = receiver.locator.partial_path().to_path_buf();
        drop(receiver);
        fs::hard_link(&partial_path, &final_path).unwrap();

        assert!(matches!(
            LinuxFileTransferReceiver::recover(
                LinuxFileTransferStateStore::for_test(state_path.clone()),
                source,
                offer.clone(),
                52
            )
            .unwrap(),
            LinuxFileTransferRecovery::AlreadyComplete
        ));
        assert_eq!(fs::read(&final_path).unwrap(), payload);
        assert!(!partial_path.exists());
        let state = LinuxFileTransferStateStore::for_test(state_path)
            .load()
            .unwrap();
        assert!(matches!(
            state.find(source, &offer),
            Ok(Some(FileTransferStateMatch::AlreadyComplete(_)))
        ));
        let _ = fs::remove_dir_all(root);
    }

    fn write_chunks(receiver: &mut LinuxFileTransferReceiver, bytes: &[u8], now: u64) {
        for chunk in bytes.chunks(FILE_TRANSFER_IO_CHUNK_BYTES) {
            receiver.write_chunk(chunk, now).unwrap();
        }
    }

    fn test_offer(tag: u8, name: &str, bytes: &[u8]) -> FileTransferOffer {
        FileTransferOffer::new(
            TransferId::from_bytes([tag; 32]),
            name.to_owned(),
            u64::try_from(bytes.len()).unwrap(),
            FileTransferDigest::from_bytes(blake3_256(bytes)),
        )
        .unwrap()
    }

    fn test_root(label: &str) -> PathBuf {
        let nonce = random_bytes::<8>().unwrap();
        let mut suffix = String::with_capacity(16);
        for byte in nonce {
            write!(&mut suffix, "{byte:02x}").unwrap();
        }
        env::temp_dir().join(format!("crosslab-file-receive-{label}-{suffix}"))
    }
}
