use core::fmt;
use std::{
    env,
    fmt::Write as _,
    fs::{self, File, OpenOptions},
    io::{Read as _, Seek as _, SeekFrom, Write as _},
    os::unix::{
        ffi::{OsStrExt as _, OsStringExt as _},
        fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _},
    },
    path::{Path, PathBuf},
};

use crosslab_agent::{
    FileTransferHasher, FileTransferLocalLocator, FileTransferStateError,
    FileTransferStateSnapshot, MAX_FILE_TRANSFER_STATE_BYTES,
};
use crosslab_protocol::{FileTransferOffer, FileTransferProfileError, TransferId};

pub const FILE_TRANSFER_IO_CHUNK_BYTES: usize = 64 * 1024;

const LOCATOR_MAGIC: &[u8; 5] = b"CLFL\x01";

#[derive(Debug)]
pub enum LinuxFileTransferError {
    HomeUnavailable,
    InvalidPath,
    InvalidSource,
    SourceChanged,
    InvalidLocator,
    State(FileTransferStateError),
    Profile(FileTransferProfileError),
    Io(std::io::Error),
}

impl fmt::Display for LinuxFileTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HomeUnavailable => {
                formatter.write_str("Linux file-transfer state home is unavailable")
            }
            Self::InvalidPath => formatter.write_str("Linux file-transfer path is invalid"),
            Self::InvalidSource => formatter.write_str("selected source is not a regular file"),
            Self::SourceChanged => {
                formatter.write_str("selected source changed during file transfer preparation")
            }
            Self::InvalidLocator => formatter.write_str("Linux file-transfer locator is invalid"),
            Self::State(error) => fmt::Display::fmt(error, formatter),
            Self::Profile(error) => fmt::Display::fmt(error, formatter),
            Self::Io(_) => formatter.write_str("Linux file-transfer storage I/O failed"),
        }
    }
}

impl std::error::Error for LinuxFileTransferError {}

impl From<std::io::Error> for LinuxFileTransferError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<FileTransferStateError> for LinuxFileTransferError {
    fn from(error: FileTransferStateError) -> Self {
        Self::State(error)
    }
}

impl From<FileTransferProfileError> for LinuxFileTransferError {
    fn from(error: FileTransferProfileError) -> Self {
        Self::Profile(error)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct LinuxFileTransferLocator {
    final_path: PathBuf,
    partial_path: PathBuf,
}

impl LinuxFileTransferLocator {
    pub fn for_destination(
        final_path: PathBuf,
        transfer_id: TransferId,
    ) -> Result<Self, LinuxFileTransferError> {
        if !valid_final_path(&final_path) {
            return Err(LinuxFileTransferError::InvalidPath);
        }
        let parent = final_path
            .parent()
            .ok_or(LinuxFileTransferError::InvalidPath)?;
        let partial_path = parent.join(partial_file_name(transfer_id));
        if partial_path == final_path {
            return Err(LinuxFileTransferError::InvalidPath);
        }
        Ok(Self {
            final_path,
            partial_path,
        })
    }

    pub fn decode(
        locator: &FileTransferLocalLocator,
        transfer_id: TransferId,
    ) -> Result<Self, LinuxFileTransferError> {
        let bytes = locator.bytes();
        if !bytes.starts_with(LOCATOR_MAGIC) {
            return Err(LinuxFileTransferError::InvalidLocator);
        }
        let mut offset = LOCATOR_MAGIC.len();
        let final_path = read_path(bytes, &mut offset)?;
        let partial_path = read_path(bytes, &mut offset)?;
        if offset != bytes.len() {
            return Err(LinuxFileTransferError::InvalidLocator);
        }

        let expected = Self::for_destination(final_path, transfer_id)?;
        if expected.partial_path != partial_path {
            return Err(LinuxFileTransferError::InvalidLocator);
        }
        Ok(expected)
    }

    pub fn encode(&self) -> Result<FileTransferLocalLocator, LinuxFileTransferError> {
        let mut bytes = Vec::with_capacity(
            LOCATOR_MAGIC.len()
                + 4
                + self.final_path.as_os_str().as_bytes().len()
                + self.partial_path.as_os_str().as_bytes().len(),
        );
        bytes.extend_from_slice(LOCATOR_MAGIC);
        push_path(&mut bytes, &self.final_path)?;
        push_path(&mut bytes, &self.partial_path)?;
        FileTransferLocalLocator::new(bytes).map_err(Into::into)
    }

    pub fn final_path(&self) -> &Path {
        &self.final_path
    }

    pub fn partial_path(&self) -> &Path {
        &self.partial_path
    }
}

impl fmt::Debug for LinuxFileTransferLocator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxFileTransferLocator")
            .field("final_path", &"[REDACTED]")
            .field("partial_path", &"[REDACTED]")
            .finish()
    }
}

pub struct LinuxFileTransferStateStore {
    path: PathBuf,
}

impl fmt::Debug for LinuxFileTransferStateStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxFileTransferStateStore")
            .field("path", &"[REDACTED]")
            .finish()
    }
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
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Default::default()),
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

pub struct LinuxPreparedFileSource {
    path: PathBuf,
    offer: FileTransferOffer,
    fingerprint: SourceFingerprint,
}

impl LinuxPreparedFileSource {
    pub fn prepare(
        path: PathBuf,
        transfer_id: TransferId,
    ) -> Result<Self, LinuxFileTransferError> {
        let display_name = source_display_name(&path)?;
        let (mut file, fingerprint) = open_regular_source(&path)?;
        let mut hasher = FileTransferHasher::new();
        let mut buffer = vec![0_u8; FILE_TRANSFER_IO_CHUNK_BYTES];

        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]).map_err(|_| LinuxFileTransferError::SourceChanged)?;
        }

        if SourceFingerprint::from_metadata(&file.metadata()?) != fingerprint {
            return Err(LinuxFileTransferError::SourceChanged);
        }
        let hash = hasher.finish();
        if hash.file_size() != fingerprint.len {
            return Err(LinuxFileTransferError::SourceChanged);
        }
        let offer = hash.into_offer(transfer_id, display_name)?;

        Ok(Self {
            path,
            offer,
            fingerprint,
        })
    }

    pub const fn offer(&self) -> &FileTransferOffer {
        &self.offer
    }

    pub fn open_reader(
        &self,
        resume_offset: u64,
    ) -> Result<LinuxFileTransferSourceReader, LinuxFileTransferError> {
        self.offer.validate_resume_offset(resume_offset)?;
        let (mut file, fingerprint) = open_regular_source(&self.path)?;
        if fingerprint != self.fingerprint {
            return Err(LinuxFileTransferError::SourceChanged);
        }
        file.seek(SeekFrom::Start(resume_offset))?;
        Ok(LinuxFileTransferSourceReader {
            file,
            expected: self.fingerprint,
            offset: resume_offset,
        })
    }
}

impl fmt::Debug for LinuxPreparedFileSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxPreparedFileSource")
            .field("path", &"[REDACTED]")
            .field("offer", &self.offer)
            .finish()
    }
}

pub struct LinuxFileTransferSourceReader {
    file: File,
    expected: SourceFingerprint,
    offset: u64,
}

impl LinuxFileTransferSourceReader {
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    pub fn read_chunk(&mut self) -> Result<Option<Vec<u8>>, LinuxFileTransferError> {
        if self.offset == self.expected.len {
            if SourceFingerprint::from_metadata(&self.file.metadata()?) != self.expected {
                return Err(LinuxFileTransferError::SourceChanged);
            }
            return Ok(None);
        }

        let remaining = self.expected.len - self.offset;
        let capacity = usize::try_from(remaining.min(FILE_TRANSFER_IO_CHUNK_BYTES as u64))
            .expect("bounded transfer chunk fits usize");
        let mut chunk = vec![0_u8; capacity];
        let read = self.file.read(&mut chunk)?;
        if read == 0 {
            return Err(LinuxFileTransferError::SourceChanged);
        }
        chunk.truncate(read);
        self.offset = self
            .offset
            .checked_add(u64::try_from(read).expect("read length fits u64"))
            .ok_or(LinuxFileTransferError::SourceChanged)?;
        Ok(Some(chunk))
    }
}

impl fmt::Debug for LinuxFileTransferSourceReader {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxFileTransferSourceReader")
            .field("offset", &self.offset)
            .field("expected_size", &self.expected.len)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceFingerprint {
    dev: u64,
    ino: u64,
    len: u64,
    mtime: i64,
    mtime_nsec: i64,
    ctime: i64,
    ctime_nsec: i64,
}

impl SourceFingerprint {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            len: metadata.len(),
            mtime: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
            ctime: metadata.ctime(),
            ctime_nsec: metadata.ctime_nsec(),
        }
    }
}

fn open_regular_source(path: &Path) -> Result<(File, SourceFingerprint), LinuxFileTransferError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(LinuxFileTransferError::InvalidSource);
    }
    let expected = SourceFingerprint::from_metadata(&metadata);
    let file = File::open(path)?;
    let opened = SourceFingerprint::from_metadata(&file.metadata()?);
    if opened != expected {
        return Err(LinuxFileTransferError::SourceChanged);
    }
    Ok((file, opened))
}

fn source_display_name(path: &Path) -> Result<String, LinuxFileTransferError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or(LinuxFileTransferError::InvalidPath)
}

fn valid_final_path(path: &Path) -> bool {
    path.is_absolute() && path.file_name().is_some() && path.parent().is_some()
}

fn partial_file_name(transfer_id: TransferId) -> String {
    let mut output = String::with_capacity(11 + 64 + 5);
    output.push_str(".crosslab-");
    for byte in transfer_id.to_bytes() {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output.push_str(".part");
    output
}

fn push_path(output: &mut Vec<u8>, path: &Path) -> Result<(), LinuxFileTransferError> {
    let bytes = path.as_os_str().as_bytes();
    let len = u16::try_from(bytes.len()).map_err(|_| LinuxFileTransferError::InvalidLocator)?;
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

fn read_path(bytes: &[u8], offset: &mut usize) -> Result<PathBuf, LinuxFileTransferError> {
    let header_end = offset
        .checked_add(2)
        .ok_or(LinuxFileTransferError::InvalidLocator)?;
    let header = bytes
        .get(*offset..header_end)
        .ok_or(LinuxFileTransferError::InvalidLocator)?;
    let len = usize::from(u16::from_be_bytes(
        header
            .try_into()
            .map_err(|_| LinuxFileTransferError::InvalidLocator)?,
    ));
    let end = header_end
        .checked_add(len)
        .ok_or(LinuxFileTransferError::InvalidLocator)?;
    let path = bytes
        .get(header_end..end)
        .ok_or(LinuxFileTransferError::InvalidLocator)?;
    if path.is_empty() {
        return Err(LinuxFileTransferError::InvalidLocator);
    }
    *offset = end;
    Ok(PathBuf::from(std::ffi::OsString::from_vec(path.to_vec())))
}

fn set_private_dir(path: &Path) -> Result<(), std::io::Error> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crosslab_agent::{
        FileTransferIdentity, FileTransferPartialState, FileTransferRetainedState,
    };
    use crosslab_crypto::{blake3_256, random_bytes};
    use crosslab_identity::DeviceId;
    use crosslab_protocol::{
        FILE_TRANSFER_CHECKPOINT_BYTES, FileTransferDigest, FileTransferOffer,
    };

    #[test]
    fn locator_round_trips_without_exposing_paths() {
        let root = test_root("locator");
        fs::create_dir_all(&root).unwrap();
        let transfer_id = TransferId::from_bytes([0x21; 32]);
        let final_path = root.join("owner-selected.bin");
        let locator =
            LinuxFileTransferLocator::for_destination(final_path.clone(), transfer_id).unwrap();
        let encoded = locator.encode().unwrap();
        let decoded = LinuxFileTransferLocator::decode(&encoded, transfer_id).unwrap();

        assert_eq!(decoded.final_path(), final_path);
        assert_eq!(decoded.partial_path(), locator.partial_path());
        let rendered = format!("{decoded:?}");
        assert!(!rendered.contains("owner-selected"));
        assert!(LinuxFileTransferLocator::decode(
            &encoded,
            TransferId::from_bytes([0x22; 32])
        )
        .is_err());

        let _ = fs::remove_dir_all(root);
    }

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
        let snapshot = FileTransferStateSnapshot::new(vec![
            FileTransferRetainedState::Partial(partial),
        ])
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
        fs::write(
            &store.path,
            vec![0_u8; MAX_FILE_TRANSFER_STATE_BYTES + 1],
        )
        .unwrap();

        assert!(matches!(
            store.load(),
            Err(LinuxFileTransferError::State(
                FileTransferStateError::SnapshotTooLarge
            ))
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepared_source_hashes_and_resumes_with_bounded_chunks() {
        let root = test_root("source");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("payload.bin");
        let len = usize::try_from(FILE_TRANSFER_CHECKPOINT_BYTES).unwrap() + 17;
        let bytes = vec![0x5a; len];
        fs::write(&path, &bytes).unwrap();

        let prepared =
            LinuxPreparedFileSource::prepare(path, TransferId::from_bytes([0x41; 32])).unwrap();
        assert_eq!(prepared.offer().file_size(), len as u64);
        assert_eq!(prepared.offer().digest().to_bytes(), blake3_256(&bytes));

        let mut reader = prepared
            .open_reader(FILE_TRANSFER_CHECKPOINT_BYTES)
            .unwrap();
        let tail = reader.read_chunk().unwrap().unwrap();
        assert_eq!(tail, bytes[len - 17..]);
        assert_eq!(reader.offset(), len as u64);
        assert_eq!(reader.read_chunk().unwrap(), None);
        assert!(!format!("{prepared:?}").contains("payload.bin"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepared_source_rejects_symlink() {
        use std::os::unix::fs::symlink;

        let root = test_root("symlink");
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.bin");
        let link = root.join("link.bin");
        fs::write(&target, b"payload").unwrap();
        symlink(&target, &link).unwrap();

        assert!(matches!(
            LinuxPreparedFileSource::prepare(link, TransferId::from_bytes([0x51; 32])),
            Err(LinuxFileTransferError::InvalidSource)
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
