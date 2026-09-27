use core::fmt;

use crosslab_crypto::Blake3Hasher;
use crosslab_protocol::{
    FileTransferDigest, FileTransferOffer, FileTransferProfileError, TransferId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferIntegrityError {
    SizeOverflow,
    SizeMismatch,
    DigestMismatch,
}

impl fmt::Display for FileTransferIntegrityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SizeOverflow => "file transfer byte count overflowed",
            Self::SizeMismatch => "file transfer byte count does not match the offer",
            Self::DigestMismatch => "file transfer digest does not match the offer",
        })
    }
}

impl std::error::Error for FileTransferIntegrityError {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FileTransferHash {
    file_size: u64,
    digest: FileTransferDigest,
}

impl FileTransferHash {
    pub const fn file_size(self) -> u64 {
        self.file_size
    }

    pub const fn digest(self) -> FileTransferDigest {
        self.digest
    }

    pub fn into_offer(
        self,
        transfer_id: TransferId,
        display_name: String,
    ) -> Result<FileTransferOffer, FileTransferProfileError> {
        FileTransferOffer::new(transfer_id, display_name, self.file_size, self.digest)
    }
}

impl fmt::Debug for FileTransferHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferHash")
            .field("file_size", &self.file_size)
            .field("digest", &"[REDACTED; 32 bytes]")
            .finish()
    }
}

pub struct FileTransferHasher {
    hasher: Blake3Hasher,
    bytes_hashed: u64,
}

impl FileTransferHasher {
    pub fn new() -> Self {
        Self {
            hasher: Blake3Hasher::new(),
            bytes_hashed: 0,
        }
    }

    pub const fn bytes_hashed(&self) -> u64 {
        self.bytes_hashed
    }

    pub fn update(&mut self, bytes: &[u8]) -> Result<(), FileTransferIntegrityError> {
        let len = u64::try_from(bytes.len()).map_err(|_| FileTransferIntegrityError::SizeOverflow)?;
        self.bytes_hashed = self
            .bytes_hashed
            .checked_add(len)
            .ok_or(FileTransferIntegrityError::SizeOverflow)?;
        self.hasher.update(bytes);
        Ok(())
    }

    pub fn finish(self) -> FileTransferHash {
        FileTransferHash {
            file_size: self.bytes_hashed,
            digest: FileTransferDigest::from_bytes(self.hasher.finalize()),
        }
    }
}

impl Default for FileTransferHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for FileTransferHasher {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferHasher")
            .field("bytes_hashed", &self.bytes_hashed)
            .finish_non_exhaustive()
    }
}

pub struct FileTransferVerifier {
    expected_size: u64,
    expected_digest: FileTransferDigest,
    hasher: FileTransferHasher,
}

impl FileTransferVerifier {
    pub fn new(offer: &FileTransferOffer) -> Self {
        Self {
            expected_size: offer.file_size(),
            expected_digest: offer.digest(),
            hasher: FileTransferHasher::new(),
        }
    }

    pub const fn bytes_hashed(&self) -> u64 {
        self.hasher.bytes_hashed()
    }

    pub fn update(&mut self, bytes: &[u8]) -> Result<(), FileTransferIntegrityError> {
        let len = u64::try_from(bytes.len()).map_err(|_| FileTransferIntegrityError::SizeOverflow)?;
        let next = self
            .hasher
            .bytes_hashed()
            .checked_add(len)
            .ok_or(FileTransferIntegrityError::SizeOverflow)?;
        if next > self.expected_size {
            return Err(FileTransferIntegrityError::SizeMismatch);
        }
        self.hasher.update(bytes)
    }

    pub fn finish(self) -> Result<(), FileTransferIntegrityError> {
        let actual = self.hasher.finish();
        if actual.file_size() != self.expected_size {
            return Err(FileTransferIntegrityError::SizeMismatch);
        }
        if actual.digest() != self.expected_digest {
            return Err(FileTransferIntegrityError::DigestMismatch);
        }
        Ok(())
    }
}

impl fmt::Debug for FileTransferVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileTransferVerifier")
            .field("expected_size", &self.expected_size)
            .field("expected_digest", &"[REDACTED; 32 bytes]")
            .field("bytes_hashed", &self.hasher.bytes_hashed())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(bytes: &[u8]) -> FileTransferOffer {
        let hash = crosslab_crypto::blake3_256(bytes);
        FileTransferOffer::new(
            TransferId::from_bytes([0x31; 32]),
            "payload.bin".into(),
            bytes.len() as u64,
            FileTransferDigest::from_bytes(hash),
        )
        .unwrap()
    }

    #[test]
    fn streaming_hash_matches_one_shot_and_builds_offer() {
        let mut hasher = FileTransferHasher::new();
        hasher.update(b"hello ").unwrap();
        hasher.update(b"world").unwrap();

        let result = hasher.finish();
        assert_eq!(result.file_size(), 11);
        assert_eq!(
            result.digest().to_bytes(),
            crosslab_crypto::blake3_256(b"hello world")
        );

        let offer = result
            .into_offer(TransferId::from_bytes([0x41; 32]), "hello.txt".into())
            .unwrap();
        assert_eq!(offer.file_size(), 11);
        assert_eq!(offer.digest(), result.digest());
    }

    #[test]
    fn verifier_accepts_exact_streamed_content() {
        let expected = offer(b"abcdef");
        let mut verifier = FileTransferVerifier::new(&expected);
        verifier.update(b"ab").unwrap();
        verifier.update(b"cd").unwrap();
        verifier.update(b"ef").unwrap();

        assert_eq!(verifier.bytes_hashed(), 6);
        assert_eq!(verifier.finish(), Ok(()));
    }

    #[test]
    fn verifier_rejects_length_and_digest_mismatch() {
        let expected = offer(b"abcdef");

        let mut short = FileTransferVerifier::new(&expected);
        short.update(b"abc").unwrap();
        assert_eq!(
            short.finish(),
            Err(FileTransferIntegrityError::SizeMismatch)
        );

        let mut long = FileTransferVerifier::new(&expected);
        assert_eq!(
            long.update(b"abcdefg"),
            Err(FileTransferIntegrityError::SizeMismatch)
        );
        assert_eq!(long.bytes_hashed(), 0);

        let mut changed = FileTransferVerifier::new(&expected);
        changed.update(b"abcdeg").unwrap();
        assert_eq!(
            changed.finish(),
            Err(FileTransferIntegrityError::DigestMismatch)
        );
    }

    #[test]
    fn integrity_debug_never_exposes_digest_or_payload() {
        let mut verifier = FileTransferVerifier::new(&offer(b"private payload"));
        verifier.update(b"private ").unwrap();

        let rendered = format!("{verifier:?}");
        assert!(!rendered.contains("private"));
        assert!(!rendered.contains(&format!(
            "{:?}",
            crosslab_crypto::blake3_256(b"private payload")
        )));
        assert!(rendered.contains("[REDACTED; 32 bytes]"));
    }
}
