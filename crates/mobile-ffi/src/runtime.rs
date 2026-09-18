use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex, MutexGuard},
    time::Duration,
};

use crosslab_runtime::RuntimeStatus;

use crate::{MobileLifecycleState, MobileRuntimeError, MobileRuntimeSnapshot};

const DEFAULT_EVENT_CAPACITY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileRuntimeEvent {
    pub snapshot: MobileRuntimeSnapshot,
}

#[derive(uniffi::Object)]
pub struct MobileRuntime {
    state: Mutex<MobileRuntimeState>,
    event_ready: Condvar,
}

struct MobileRuntimeState {
    snapshot: MobileRuntimeSnapshot,
    events: VecDeque<MobileRuntimeEvent>,
    event_capacity: usize,
}

#[uniffi::export]
impl MobileRuntime {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self) -> Result<(), MobileRuntimeError> {
        let mut state = self.lock_state()?;
        if state.snapshot.lifecycle == MobileLifecycleState::Running {
            return Err(MobileRuntimeError::AlreadyStarted);
        }

        let revision = next_revision(state.snapshot.revision)?;
        state.events.clear();
        replace_snapshot(
            &mut state,
            MobileRuntimeSnapshot::disconnected(MobileLifecycleState::Running, revision),
        );
        self.event_ready.notify_all();
        Ok(())
    }

    pub fn stop(&self) -> Result<(), MobileRuntimeError> {
        let mut state = self.lock_state()?;
        if state.snapshot.lifecycle == MobileLifecycleState::Stopped {
            return Err(MobileRuntimeError::NotStarted);
        }

        let revision = next_revision(state.snapshot.revision)?;
        state.events.clear();
        replace_snapshot(
            &mut state,
            MobileRuntimeSnapshot::disconnected(MobileLifecycleState::Stopped, revision),
        );
        self.event_ready.notify_all();
        Ok(())
    }

    pub fn network_lost(&self) -> Result<(), MobileRuntimeError> {
        let mut state = self.lock_state()?;
        if state.snapshot.lifecycle != MobileLifecycleState::Running {
            return Err(MobileRuntimeError::NotStarted);
        }

        let revision = next_revision(state.snapshot.revision)?;
        replace_snapshot(
            &mut state,
            MobileRuntimeSnapshot::disconnected(MobileLifecycleState::Running, revision),
        );
        self.event_ready.notify_all();
        Ok(())
    }

    pub fn network_available(&self) -> Result<(), MobileRuntimeError> {
        let state = self.lock_state()?;
        if state.snapshot.lifecycle != MobileLifecycleState::Running {
            return Err(MobileRuntimeError::NotStarted);
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Result<MobileRuntimeSnapshot, MobileRuntimeError> {
        Ok(self.lock_state()?.snapshot.clone())
    }

    pub fn poll_event(&self) -> Result<Option<MobileRuntimeEvent>, MobileRuntimeError> {
        Ok(self.lock_state()?.events.pop_front())
    }

    pub fn wait_event(
        &self,
        timeout_ms: u64,
    ) -> Result<Option<MobileRuntimeEvent>, MobileRuntimeError> {
        let state = self.lock_state()?;
        let (mut state, _) = self
            .event_ready
            .wait_timeout_while(state, Duration::from_millis(timeout_ms), |state| {
                state.events.is_empty()
            })
            .map_err(|_| MobileRuntimeError::StateUnavailable)?;
        Ok(state.events.pop_front())
    }
}

impl MobileRuntime {
    /// Rust-side runtime bridge; this method is not exported through UniFFI.
    pub fn publish_runtime_status(&self, status: &RuntimeStatus) -> Result<(), MobileRuntimeError> {
        let mut state = self.lock_state()?;
        if state.snapshot.lifecycle != MobileLifecycleState::Running {
            return Err(MobileRuntimeError::NotStarted);
        }

        let revision = next_revision(state.snapshot.revision)?;
        let snapshot =
            MobileRuntimeSnapshot::from_runtime(status, MobileLifecycleState::Running, revision);
        replace_snapshot(&mut state, snapshot);
        self.event_ready.notify_all();
        Ok(())
    }
}

impl Default for MobileRuntime {
    fn default() -> Self {
        Self::with_event_capacity(DEFAULT_EVENT_CAPACITY)
    }
}

impl MobileRuntime {
    fn with_event_capacity(event_capacity: usize) -> Self {
        assert!(event_capacity > 0, "event capacity must be non-zero");
        Self {
            state: Mutex::new(MobileRuntimeState {
                snapshot: MobileRuntimeSnapshot::disconnected(MobileLifecycleState::Stopped, 0),
                events: VecDeque::with_capacity(event_capacity),
                event_capacity,
            }),
            event_ready: Condvar::new(),
        }
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, MobileRuntimeState>, MobileRuntimeError> {
        self.state
            .lock()
            .map_err(|_| MobileRuntimeError::StateUnavailable)
    }

    #[cfg(test)]
    pub(crate) fn with_test_event_capacity(event_capacity: usize) -> Self {
        Self::with_event_capacity(event_capacity)
    }

    #[cfg(test)]
    pub(crate) fn publish_test_snapshot(
        &self,
        snapshot: MobileRuntimeSnapshot,
    ) -> Result<(), MobileRuntimeError> {
        let mut state = self.lock_state()?;
        replace_snapshot(&mut state, snapshot);
        self.event_ready.notify_all();
        Ok(())
    }
}

fn replace_snapshot(state: &mut MobileRuntimeState, snapshot: MobileRuntimeSnapshot) {
    if state.events.len() == state.event_capacity {
        state.events.pop_front();
    }
    state.events.push_back(MobileRuntimeEvent {
        snapshot: snapshot.clone(),
    });
    state.snapshot = snapshot;
}

fn next_revision(current: u64) -> Result<u64, MobileRuntimeError> {
    current
        .checked_add(1)
        .ok_or(MobileRuntimeError::StateUnavailable)
}
