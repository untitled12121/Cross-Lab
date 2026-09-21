use std::sync::Mutex;

use crate::{
    IdentityStoreAnchor, IdentityStoreEnvelope, IdentityStoreError, prepare_commit, validate_loaded,
};

#[derive(Debug, Default)]
pub struct MemoryIdentityStore {
    state: Mutex<MemoryState>,
}

#[derive(Debug, Default)]
struct MemoryState {
    envelope: Option<IdentityStoreEnvelope>,
    anchor: Option<IdentityStoreAnchor>,
}

impl MemoryIdentityStore {
    pub fn load(&self) -> Result<Option<IdentityStoreEnvelope>, IdentityStoreError> {
        let state = self
            .state
            .lock()
            .expect("memory identity-store lock poisoned");
        match (&state.envelope, state.anchor) {
            (None, None) => Ok(None),
            (Some(envelope), Some(anchor)) => {
                validate_loaded(envelope, anchor)?;
                Ok(Some(envelope.clone()))
            }
            _ => Err(IdentityStoreError::StaleOrMixedState),
        }
    }

    pub fn compare_and_swap(
        &self,
        expected_revision: Option<u64>,
        payload: Vec<u8>,
    ) -> Result<IdentityStoreEnvelope, IdentityStoreError> {
        let mut state = self
            .state
            .lock()
            .expect("memory identity-store lock poisoned");
        let current_revision = state.envelope.as_ref().map(IdentityStoreEnvelope::revision);
        if current_revision != expected_revision {
            return Err(IdentityStoreError::RevisionConflict);
        }

        let commit = prepare_commit(state.envelope.as_ref(), payload)?;
        state.envelope = Some(commit.envelope().clone());
        state.anchor = Some(commit.anchor());
        Ok(commit.envelope().clone())
    }

    #[cfg(test)]
    pub(crate) fn replace_anchor_for_test(&self, anchor: IdentityStoreAnchor) {
        self.state
            .lock()
            .expect("memory identity-store lock poisoned")
            .anchor = Some(anchor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_and_swap_rejects_stale_writer() {
        let store = MemoryIdentityStore::default();
        let first = store.compare_and_swap(None, b"one".to_vec()).unwrap();

        assert_eq!(
            store.compare_and_swap(None, b"stale".to_vec()),
            Err(IdentityStoreError::RevisionConflict)
        );

        let second = store
            .compare_and_swap(Some(first.revision()), b"two".to_vec())
            .unwrap();
        assert_eq!(second.revision(), 2);
    }

    #[test]
    fn load_fails_closed_when_anchor_is_rolled_back() {
        let store = MemoryIdentityStore::default();
        let first = store.compare_and_swap(None, b"one".to_vec()).unwrap();
        let first_anchor = IdentityStoreAnchor::new(first.revision(), first.envelope_digest());
        let _ = store
            .compare_and_swap(Some(first.revision()), b"two".to_vec())
            .unwrap();

        store.replace_anchor_for_test(first_anchor);
        assert_eq!(store.load(), Err(IdentityStoreError::StaleOrMixedState));
    }
}
