use core::fmt;
use std::sync::{Arc, Mutex};

use crosslab_agent::{
    FileTransferCompletionTombstone, FileTransferIdentity, FileTransferLocalLocator,
    FileTransferPartialState, FileTransferRetainedState, FileTransferStateMatch,
    FileTransferStateSnapshot,
};

use super::{MobileFileTransferError, MobileFileTransferOffer, device_id, transfer_id};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileExpiredFileTransfer {
    pub transfer_id: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileFileTransferRecoveryKind {
    None,
    Partial,
    AlreadyComplete,
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferRecovery {
    kind: MobileFileTransferRecoveryKind,
    durable_offset: Option<u64>,
    locator: Option<Vec<u8>>,
}

impl fmt::Debug for MobileFileTransferRecovery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MobileFileTransferRecovery")
            .field("kind", &self.kind)
            .field("durable_offset", &self.durable_offset)
            .field(
                "locator",
                &self
                    .locator
                    .as_ref()
                    .map(|locator| format!("[REDACTED; {} bytes]", locator.len())),
            )
            .finish()
    }
}

#[uniffi::export]
impl MobileFileTransferRecovery {
    pub fn kind(&self) -> MobileFileTransferRecoveryKind {
        self.kind
    }

    pub fn durable_offset(&self) -> Option<u64> {
        self.durable_offset
    }

    pub fn locator(&self) -> Option<Vec<u8>> {
        self.locator.clone()
    }
}

#[derive(uniffi::Object)]
pub struct MobileFileTransferState {
    state: Mutex<FileTransferStateSnapshot>,
}

#[uniffi::export]
impl MobileFileTransferState {
    #[uniffi::constructor]
    pub fn new(encoded: Option<Vec<u8>>) -> Result<Self, MobileFileTransferError> {
        let state = match encoded {
            Some(encoded) => FileTransferStateSnapshot::decode(&encoded)?,
            None => FileTransferStateSnapshot::default(),
        };
        Ok(Self {
            state: Mutex::new(state),
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, MobileFileTransferError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?
            .encode()?)
    }

    pub fn find(
        &self,
        source_device_id_bytes: Vec<u8>,
        offer: Arc<MobileFileTransferOffer>,
    ) -> Result<Arc<MobileFileTransferRecovery>, MobileFileTransferError> {
        let source_device_id = device_id(source_device_id_bytes)?;
        let state = self
            .state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        let recovery = match state.find(source_device_id, offer.offer())? {
            None => MobileFileTransferRecovery {
                kind: MobileFileTransferRecoveryKind::None,
                durable_offset: None,
                locator: None,
            },
            Some(FileTransferStateMatch::Partial(partial)) => MobileFileTransferRecovery {
                kind: MobileFileTransferRecoveryKind::Partial,
                durable_offset: Some(partial.durable_offset()),
                locator: Some(partial.locator().bytes().to_vec()),
            },
            Some(FileTransferStateMatch::AlreadyComplete(_)) => MobileFileTransferRecovery {
                kind: MobileFileTransferRecoveryKind::AlreadyComplete,
                durable_offset: Some(offer.file_size()),
                locator: None,
            },
        };
        Ok(Arc::new(recovery))
    }

    pub fn upsert_partial(
        &self,
        source_device_id_bytes: Vec<u8>,
        offer: Arc<MobileFileTransferOffer>,
        durable_offset: u64,
        locator: Vec<u8>,
        updated_at_unix_secs: u64,
    ) -> Result<(), MobileFileTransferError> {
        let identity =
            FileTransferIdentity::new(device_id(source_device_id_bytes)?, offer.offer().clone());
        let partial = FileTransferPartialState::new(
            identity,
            durable_offset,
            FileTransferLocalLocator::new(locator)?,
            updated_at_unix_secs,
        )?;
        self.state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?
            .upsert_partial(partial)?;
        Ok(())
    }

    pub fn mark_completed(
        &self,
        source_device_id_bytes: Vec<u8>,
        offer: Arc<MobileFileTransferOffer>,
        updated_at_unix_secs: u64,
    ) -> Result<(), MobileFileTransferError> {
        let identity =
            FileTransferIdentity::new(device_id(source_device_id_bytes)?, offer.offer().clone());
        let completed = FileTransferCompletionTombstone::new(identity, updated_at_unix_secs);
        self.state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?
            .mark_completed(completed)?;
        Ok(())
    }

    pub fn remove(&self, transfer_id_bytes: Vec<u8>) -> Result<bool, MobileFileTransferError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?
            .remove(transfer_id(transfer_id_bytes)?))
    }

    pub fn cleanup_expired(
        &self,
        now_unix_secs: u64,
        partial_max_age_secs: u64,
        completion_max_age_secs: u64,
    ) -> Result<Vec<MobileExpiredFileTransfer>, MobileFileTransferError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| MobileFileTransferError::StateUnavailable)?;
        let expired = state
            .entries()
            .iter()
            .filter(|entry| {
                let max_age = match entry {
                    FileTransferRetainedState::Partial(_) => partial_max_age_secs,
                    FileTransferRetainedState::Completed(_) => completion_max_age_secs,
                };
                now_unix_secs >= entry.updated_at_unix_secs()
                    && now_unix_secs.saturating_sub(entry.updated_at_unix_secs()) > max_age
            })
            .cloned()
            .collect::<Vec<_>>();
        let partial_ids = expired
            .iter()
            .filter_map(|entry| match entry {
                FileTransferRetainedState::Partial(partial) => Some(MobileExpiredFileTransfer {
                    transfer_id: partial.identity().transfer_id().to_bytes().to_vec(),
                }),
                FileTransferRetainedState::Completed(_) => None,
            })
            .collect::<Vec<_>>();
        for entry in expired {
            state.remove(entry.identity().transfer_id());
        }
        Ok(partial_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_transfer::MobileFileTransferHasher;

    #[test]
    fn cleanup_expires_partials_before_completion_tombstones() {
        let partial_hasher = MobileFileTransferHasher::new();
        partial_hasher.update(b"partial".to_vec()).unwrap();
        let partial_offer = partial_hasher.finish_new("partial.bin".into()).unwrap();

        let completed_hasher = MobileFileTransferHasher::new();
        completed_hasher.update(b"complete".to_vec()).unwrap();
        let completed_offer = completed_hasher.finish_new("complete.bin".into()).unwrap();

        let state = MobileFileTransferState::new(None).unwrap();
        state
            .upsert_partial(
                vec![0x51; 32],
                Arc::clone(&partial_offer),
                0,
                b"content://private/partial".to_vec(),
                10,
            )
            .unwrap();
        state
            .mark_completed(vec![0x52; 32], Arc::clone(&completed_offer), 10)
            .unwrap();

        let expired = state.cleanup_expired(21, 10, 20).unwrap();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].transfer_id, partial_offer.transfer_id());

        assert_eq!(
            state.find(vec![0x51; 32], partial_offer).unwrap().kind(),
            MobileFileTransferRecoveryKind::None
        );
        assert_eq!(
            state.find(vec![0x52; 32], completed_offer).unwrap().kind(),
            MobileFileTransferRecoveryKind::AlreadyComplete
        );
    }

    #[test]
    fn retained_state_round_trips_redacted_locator() {
        let hasher = MobileFileTransferHasher::new();
        hasher.update(b"abc".to_vec()).unwrap();
        let offer = hasher.finish_new("a.bin".into()).unwrap();
        let state = MobileFileTransferState::new(None).unwrap();
        state
            .upsert_partial(
                vec![0x41; 32],
                Arc::clone(&offer),
                0,
                b"content://private/document".to_vec(),
                10,
            )
            .unwrap();

        let encoded = state.encode().unwrap();
        let restored = MobileFileTransferState::new(Some(encoded)).unwrap();
        let recovery = restored.find(vec![0x41; 32], offer).unwrap();

        assert_eq!(recovery.kind(), MobileFileTransferRecoveryKind::Partial);
        assert_eq!(recovery.durable_offset(), Some(0));
        assert_eq!(
            recovery.locator().unwrap(),
            b"content://private/document".to_vec()
        );
        assert!(!format!("{recovery:?}").contains("content://"));
    }
}
