use core::fmt;
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
};

use crosslab_agent::{
    FileTransferRetainedState, FileTransferStateError, FileTransferStateSnapshot,
    MAX_FILE_TRANSFER_STATE_BYTES,
};

use super::{LinuxFileTransferError, LinuxFileTransferLocator};

const PARTIAL_RETENTION_SECS: u64 = 7 * 24 * 60 * 60;
const COMPLETION_RETENTION_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Clone)]
pub struct LinuxFileTransferStateStore {
    path: PathBuf,
}

impl LinuxFileTransferStateStore {
    pub fn from_environment() -> Result<Self, LinuxFileTransferError> {
        let base = if let Some(state_home) = env::var_os("XDG_STATE_HOME") {
            PathBuf::from(state_home)
        } else {
            let home = env::var_os("HOME").ok_or(LinuxFileTransferError::HomeUnavailable)?;
            PathBuf::from(home).join(".local/state")
        };
        Ok(Self {
            path: base.join("crosslab/file-transfer/state-v1.bin"),
        })
    }

    pub fn load(&self) -> Result<FileTransferStateSnapshot, LinuxFileTransferError> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Default::default());
            }
            Err(error) => return Err(error.into()),
        };
        let max = u64::try_from(MAX_FILE_TRANSFER_STATE_BYTES)
            .expect("file-transfer state bound fits u64");
        if file.metadata()?.len() > max {
            return Err(FileTransferStateError::SnapshotTooLarge.into());
        }

        let mut bytes = Vec::new();
        (&mut file).take(max + 1).read_to_end(&mut bytes)?;
        if bytes.len() > MAX_FILE_TRANSFER_STATE_BYTES {
            return Err(FileTransferStateError::SnapshotTooLarge.into());
        }
        FileTransferStateSnapshot::decode(&bytes).map_err(Into::into)
    }

    pub fn cleanup_expired(&self, now_unix_secs: u64) -> Result<usize, LinuxFileTransferError> {
        let mut snapshot = self.load()?;
        let expired = snapshot
            .entries()
            .iter()
            .filter(|entry| {
                let max_age = match entry {
                    FileTransferRetainedState::Partial(_) => PARTIAL_RETENTION_SECS,
                    FileTransferRetainedState::Completed(_) => COMPLETION_RETENTION_SECS,
                };
                entry_expired(entry.updated_at_unix_secs(), now_unix_secs, max_age)
            })
            .cloned()
            .collect::<Vec<_>>();

        for entry in &expired {
            if let FileTransferRetainedState::Partial(partial) = entry {
                let locator = LinuxFileTransferLocator::decode(
                    partial.locator(),
                    partial.identity().transfer_id(),
                )?;
                remove_expired_partial(locator.partial_path())?;
            }
            snapshot.remove(entry.identity().transfer_id());
        }

        if !expired.is_empty() {
            self.commit(&snapshot)?;
        }
        Ok(expired.len())
    }

    pub fn commit(
        &self,
        snapshot: &FileTransferStateSnapshot,
    ) -> Result<(), LinuxFileTransferError> {
        let bytes = snapshot.encode()?;
        let parent = self
            .path
            .parent()
            .ok_or(LinuxFileTransferError::InvalidPath)?;
        fs::create_dir_all(parent)?;
        set_private_dir(parent)?;

        let temp = self.path.with_extension("bin.new");
        match fs::remove_file(&temp) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        let write_result = (|| -> Result<(), std::io::Error> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()
        })();
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temp);
            return Err(error.into());
        }

        fs::rename(&temp, &self.path)?;
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600))?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn for_test(path: PathBuf) -> Self {
        Self { path }
    }
}

impl fmt::Debug for LinuxFileTransferStateStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxFileTransferStateStore")
            .field("path", &"[REDACTED]")
            .finish()
    }
}

fn set_private_dir(path: &Path) -> Result<(), std::io::Error> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

fn entry_expired(updated_at: u64, now: u64, max_age: u64) -> bool {
    now >= updated_at && now.saturating_sub(updated_at) > max_age
}

fn remove_expired_partial(path: &Path) -> Result<(), LinuxFileTransferError> {
    match fs::remove_file(path) {
        Ok(()) => {
            if let Some(parent) = path.parent() {
                File::open(parent)?.sync_all()?;
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use crosslab_agent::{
        FileTransferCompletionTombstone, FileTransferIdentity, FileTransferPartialState,
        FileTransferRetainedState,
    };
    use crosslab_crypto::{blake3_256, random_bytes};
    use crosslab_identity::DeviceId;
    use crosslab_protocol::{FileTransferDigest, FileTransferOffer, TransferId};

    use super::super::LinuxFileTransferLocator;
    use super::*;

    #[test]
    fn state_store_round_trips_private_snapshot() {
        let root = test_root("state");
        let store = LinuxFileTransferStateStore::for_test(root.join("state-v1.bin"));
        let transfer_id = TransferId::from_bytes([0x31; 32]);
        let offer = test_offer(transfer_id, "state.bin", b"state");
        let locator =
            LinuxFileTransferLocator::for_destination(root.join("selected.bin"), transfer_id)
                .unwrap()
                .encode()
                .unwrap();
        let partial = FileTransferPartialState::new(
            FileTransferIdentity::new(DeviceId::from_bytes([0x32; 32]), offer),
            0,
            locator,
            10,
        )
        .unwrap();
        let snapshot =
            FileTransferStateSnapshot::new(vec![FileTransferRetainedState::Partial(partial)])
                .unwrap();

        store.commit(&snapshot).unwrap();
        assert_eq!(store.load().unwrap(), snapshot);
        assert_eq!(
            fs::metadata(&store.path).unwrap().permissions().mode() & 0o077,
            0
        );
        assert_eq!(
            fs::metadata(store.path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
        assert!(!format!("{store:?}").contains(root.to_string_lossy().as_ref()));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cleanup_expires_partial_files_and_completion_tombstones() {
        let root = test_root("cleanup");
        fs::create_dir_all(&root).unwrap();
        let store = LinuxFileTransferStateStore::for_test(root.join("state-v1.bin"));

        let partial_id = TransferId::from_bytes([0x41; 32]);
        let partial_locator =
            LinuxFileTransferLocator::for_destination(root.join("partial.bin"), partial_id).unwrap();
        fs::write(partial_locator.partial_path(), b"partial").unwrap();
        let partial = FileTransferPartialState::new(
            FileTransferIdentity::new(
                DeviceId::from_bytes([0x42; 32]),
                test_offer(partial_id, "partial.bin", b"partial"),
            ),
            0,
            partial_locator.encode().unwrap(),
            10,
        )
        .unwrap();

        let complete_id = TransferId::from_bytes([0x43; 32]);
        let completed = FileTransferCompletionTombstone::new(
            FileTransferIdentity::new(
                DeviceId::from_bytes([0x44; 32]),
                test_offer(complete_id, "complete.bin", b"complete"),
            ),
            10,
        );
        store
            .commit(
                &FileTransferStateSnapshot::new(vec![
                    FileTransferRetainedState::Partial(partial),
                    FileTransferRetainedState::Completed(completed),
                ])
                .unwrap(),
            )
            .unwrap();

        let now = 10 + COMPLETION_RETENTION_SECS + 1;
        assert_eq!(store.cleanup_expired(now).unwrap(), 2);
        assert!(!partial_locator.partial_path().exists());
        assert!(store.load().unwrap().is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cleanup_keeps_recent_and_future_dated_entries() {
        assert!(!entry_expired(100, 100, 10));
        assert!(!entry_expired(95, 100, 10));
        assert!(!entry_expired(110, 100, 10));
        assert!(entry_expired(89, 100, 10));
    }

    #[test]
    fn state_store_rejects_oversized_input_before_decode() {
        let root = test_root("oversized");
        fs::create_dir_all(&root).unwrap();
        let store = LinuxFileTransferStateStore::for_test(root.join("state-v1.bin"));
        fs::write(&store.path, vec![0_u8; MAX_FILE_TRANSFER_STATE_BYTES + 1]).unwrap();

        assert!(matches!(
            store.load(),
            Err(LinuxFileTransferError::State(
                FileTransferStateError::SnapshotTooLarge
            ))
        ));

        let _ = fs::remove_dir_all(root);
    }

    fn test_offer(transfer_id: TransferId, name: &str, bytes: &[u8]) -> FileTransferOffer {
        FileTransferOffer::new(
            transfer_id,
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
        env::temp_dir().join(format!("crosslab-file-transfer-{label}-{suffix}"))
    }
}
