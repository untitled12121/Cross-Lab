use core::fmt;
use std::{
    fs::{self, File},
    io::{Read as _, Seek as _, SeekFrom},
    os::unix::fs::MetadataExt as _,
    path::{Path, PathBuf},
};

use crosslab_agent::FileTransferHasher;
use crosslab_protocol::{FileTransferOffer, TransferId};

use super::LinuxFileTransferError;

pub const FILE_TRANSFER_IO_CHUNK_BYTES: usize = 64 * 1024;

pub struct LinuxPreparedFileSource {
    path: PathBuf,
    offer: FileTransferOffer,
    fingerprint: SourceFingerprint,
}

impl LinuxPreparedFileSource {
    pub fn prepare(path: PathBuf, transfer_id: TransferId) -> Result<Self, LinuxFileTransferError> {
        let display_name = source_display_name(&path)?;
        let (mut file, fingerprint) = open_regular_source(&path)?;
        let mut hasher = FileTransferHasher::new();
        let mut buffer = vec![0_u8; FILE_TRANSFER_IO_CHUNK_BYTES];

        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher
                .update(&buffer[..read])
                .map_err(|_| LinuxFileTransferError::SourceChanged)?;
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
        let chunk_bound =
            u64::try_from(FILE_TRANSFER_IO_CHUNK_BYTES).expect("transfer chunk bound fits u64");
        let capacity =
            usize::try_from(remaining.min(chunk_bound)).expect("bounded transfer chunk fits usize");
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

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use crosslab_crypto::{blake3_256, random_bytes};
    use crosslab_protocol::FILE_TRANSFER_CHECKPOINT_BYTES;

    use super::*;

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

    fn test_root(label: &str) -> PathBuf {
        let nonce = random_bytes::<8>().unwrap();
        let mut suffix = String::with_capacity(16);
        use std::fmt::Write as _;
        for byte in nonce {
            write!(&mut suffix, "{byte:02x}").unwrap();
        }
        env::temp_dir().join(format!("crosslab-file-transfer-{label}-{suffix}"))
    }
}
