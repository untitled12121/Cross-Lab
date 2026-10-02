use core::fmt;
use std::sync::{Arc, Mutex};

use crosslab_agent::{FileTransferHash, FileTransferHasher, FileTransferVerifier};
use crosslab_protocol::{FileTransferOffer, TransferId};

use super::{MobileFileTransferError, digest, transfer_id};

#[derive(uniffi::Object)]
pub struct MobileFileTransferOffer {
    offer: FileTransferOffer,
}

impl MobileFileTransferOffer {
    pub(crate) fn from_offer(offer: FileTransferOffer) -> Self {
        Self { offer }
    }

    pub(crate) fn offer(&self) -> &FileTransferOffer {
        &self.offer
    }
}

impl fmt::Debug for MobileFileTransferOffer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MobileFileTransferOffer")
            .field("transfer_id", &self.offer.transfer_id())
            .field("display_name_len", &self.offer.display_name().len())
            .field("file_size", &self.offer.file_size())
            .field("digest", &"[REDACTED; 32 bytes]")
            .finish()
    }
}

#[uniffi::export]
impl MobileFileTransferOffer {
    #[uniffi::constructor]
    pub fn from_parts(
        transfer_id_bytes: Vec<u8>,
        display_name: String,
        file_size: u64,
        digest_bytes: Vec<u8>,
    ) -> Result<Self, MobileFileTransferError> {
        let offer = FileTransferOffer::new(
            transfer_id(transfer_id_bytes)?,
            display_name,
            file_size,
            digest(digest_bytes)?,
        )?;
        Ok(Self { offer })
    }

    pub fn transfer_id(&self) -> Vec<u8> {
        self.offer.transfer_id().to_bytes().to_vec()
    }

    pub fn display_name(&self) -> String {
        self.offer.display_name().to_owned()
    }

    pub fn file_size(&self) -> u64 {
        self.offer.file_size()
    }

    pub fn digest(&self) -> Vec<u8> {
        self.offer.digest().to_bytes().to_vec()
    }

    pub fn same_identity(&self, other: Arc<MobileFileTransferOffer>) -> bool {
        self.offer == other.offer
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferHasher {
    inner: Mutex<Option<FileTransferHasher>>,
}

#[uniffi::export]
impl MobileFileTransferHasher {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Some(FileTransferHasher::new())),
        }
    }

    pub fn update(&self, bytes: Vec<u8>) -> Result<(), MobileFileTransferError> {
        let mut hasher = self
            .inner
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        hasher
            .as_mut()
            .ok_or(MobileFileTransferError::StateUnavailable)?
            .update(&bytes)?;
        Ok(())
    }

    pub fn bytes_hashed(&self) -> Result<u64, MobileFileTransferError> {
        let hasher = self
            .inner
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        Ok(hasher
            .as_ref()
            .ok_or(MobileFileTransferError::StateUnavailable)?
            .bytes_hashed())
    }

    pub fn finish_new(
        &self,
        display_name: String,
    ) -> Result<Arc<MobileFileTransferOffer>, MobileFileTransferError> {
        let transfer_id = TransferId::generate().map_err(|_| MobileFileTransferError::Random)?;
        self.finish_with_id(transfer_id, display_name)
    }

    pub fn finish_existing(
        &self,
        transfer_id_bytes: Vec<u8>,
        display_name: String,
    ) -> Result<Arc<MobileFileTransferOffer>, MobileFileTransferError> {
        self.finish_with_id(transfer_id(transfer_id_bytes)?, display_name)
    }
}

impl MobileFileTransferHasher {
    fn finish_with_id(
        &self,
        transfer_id: TransferId,
        display_name: String,
    ) -> Result<Arc<MobileFileTransferOffer>, MobileFileTransferError> {
        let mut hasher = self
            .inner
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        let hash: FileTransferHash = hasher
            .take()
            .ok_or(MobileFileTransferError::StateUnavailable)?
            .finish();
        let offer = hash.into_offer(transfer_id, display_name)?;
        Ok(Arc::new(MobileFileTransferOffer::from_offer(offer)))
    }
}

impl Default for MobileFileTransferHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferVerifier {
    inner: Mutex<Option<FileTransferVerifier>>,
}

#[uniffi::export]
impl MobileFileTransferVerifier {
    #[uniffi::constructor]
    pub fn new(offer: Arc<MobileFileTransferOffer>) -> Self {
        Self {
            inner: Mutex::new(Some(FileTransferVerifier::new(offer.offer()))),
        }
    }

    pub fn update(&self, bytes: Vec<u8>) -> Result<(), MobileFileTransferError> {
        let mut verifier = self
            .inner
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        verifier
            .as_mut()
            .ok_or(MobileFileTransferError::StateUnavailable)?
            .update(&bytes)?;
        Ok(())
    }

    pub fn bytes_hashed(&self) -> Result<u64, MobileFileTransferError> {
        let verifier = self
            .inner
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        Ok(verifier
            .as_ref()
            .ok_or(MobileFileTransferError::StateUnavailable)?
            .bytes_hashed())
    }

    pub fn finish(&self) -> Result<(), MobileFileTransferError> {
        let mut verifier = self
            .inner
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        verifier
            .take()
            .ok_or(MobileFileTransferError::StateUnavailable)?
            .finish()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_hash_and_verify_round_trip_without_payload_debug() {
        let hasher = MobileFileTransferHasher::new();
        hasher.update(b"private ".to_vec()).unwrap();
        hasher.update(b"payload".to_vec()).unwrap();
        let offer = hasher.finish_new("payload.bin".into()).unwrap();

        let verifier = MobileFileTransferVerifier::new(Arc::clone(&offer));
        verifier.update(b"private ".to_vec()).unwrap();
        verifier.update(b"payload".to_vec()).unwrap();
        verifier.finish().unwrap();

        let rendered = format!("{offer:?}");
        assert!(!rendered.contains("payload.bin"));
        assert!(!rendered.contains("private"));
    }

    #[test]
    fn retry_hash_keeps_transfer_id_only_for_exact_source_identity() {
        let first = MobileFileTransferHasher::new();
        first.update(b"same bytes".to_vec()).unwrap();
        let offer = first.finish_new("same.bin".into()).unwrap();

        let retry = MobileFileTransferHasher::new();
        retry.update(b"same bytes".to_vec()).unwrap();
        let matching = retry
            .finish_existing(offer.transfer_id(), "same.bin".into())
            .unwrap();
        assert!(matching.same_identity(Arc::clone(&offer)));

        let changed = MobileFileTransferHasher::new();
        changed.update(b"changed".to_vec()).unwrap();
        let changed = changed
            .finish_existing(offer.transfer_id(), "same.bin".into())
            .unwrap();
        assert!(!changed.same_identity(offer));
    }
}
