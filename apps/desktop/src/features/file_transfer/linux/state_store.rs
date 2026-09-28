use core::fmt;
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
};

use crosslab_agent::{
    FileTransferStateError, FileTransferStateSnapshot, MAX_FILE_TRANSFER_STATE_BYTES,
};

use super::LinuxFileTransferError;

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
    fn for_test(path: PathBuf) -> Self {
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

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use crosslab_agent::{
        FileTransferIdentity, FileTransferPartialState, FileTransferRetainedState,
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
        assert_eq!(fs::metadata(&store.path).unwrap().mode() & 0o077, 0);
        assert_eq!(
            fs::metadata(store.path.parent().unwrap()).unwrap().mode() & 0o077,
            0
        );
        assert!(!format!("{store:?}").contains(root.to_string_lossy().as_ref()));

        let _ = fs::remove_dir_all(root);
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
