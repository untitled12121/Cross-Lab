use std::{
    num::{NonZeroU32, NonZeroU64, NonZeroUsize},
    time::Duration,
};

const DEFAULT_CONTROL_QUEUE_CAPACITY: usize = 8;
const DEFAULT_INCOMING_STREAM_QUEUE_CAPACITY: usize = 8;
const DEFAULT_OUTGOING_STREAM_CAPACITY: usize = 8;
const DEFAULT_STREAM_CHUNK_QUEUE_CAPACITY: usize = 8;
const DEFAULT_MAX_CONTROL_FRAME_BYTES: usize = 256 * 1024 + 4;
const DEFAULT_MAX_OPENING_FRAME_BYTES: usize = 4 * 1024 + 4;
const DEFAULT_MAX_CHUNK_BYTES: usize = 64 * 1024;
const DEFAULT_MAX_CONCURRENT_REMOTE_UNI_STREAMS: u32 = 32;
const DEFAULT_MAX_CONCURRENT_REMOTE_BI_STREAMS: u32 = 1;
const DEFAULT_STREAM_RECEIVE_WINDOW: u64 = 512 * 1024;
const DEFAULT_CONNECTION_RECEIVE_WINDOW: u64 = 4 * 1024 * 1024;
const DEFAULT_IDLE_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateConfig {
    control_queue_capacity: NonZeroUsize,
    incoming_stream_queue_capacity: NonZeroUsize,
    outgoing_stream_capacity: NonZeroUsize,
    stream_chunk_queue_capacity: NonZeroUsize,
    max_control_frame_bytes: NonZeroUsize,
    max_opening_frame_bytes: NonZeroUsize,
    max_chunk_bytes: NonZeroUsize,
    max_concurrent_remote_uni_streams: NonZeroU32,
    max_concurrent_remote_bi_streams: NonZeroU32,
    stream_receive_window: NonZeroU64,
    connection_receive_window: NonZeroU64,
    idle_timeout_ms: NonZeroU64,
}

impl CandidateConfig {
    pub const fn with_control_limits(
        mut self,
        control_queue_capacity: NonZeroUsize,
        max_control_frame_bytes: NonZeroUsize,
    ) -> Self {
        self.control_queue_capacity = control_queue_capacity;
        self.max_control_frame_bytes = max_control_frame_bytes;
        self
    }

    pub const fn control_queue_capacity(self) -> usize {
        self.control_queue_capacity.get()
    }

    pub const fn incoming_stream_queue_capacity(self) -> usize {
        self.incoming_stream_queue_capacity.get()
    }

    pub const fn outgoing_stream_capacity(self) -> usize {
        self.outgoing_stream_capacity.get()
    }

    pub const fn stream_chunk_queue_capacity(self) -> usize {
        self.stream_chunk_queue_capacity.get()
    }

    pub const fn max_control_frame_bytes(self) -> usize {
        self.max_control_frame_bytes.get()
    }

    pub const fn max_opening_frame_bytes(self) -> usize {
        self.max_opening_frame_bytes.get()
    }

    pub const fn max_chunk_bytes(self) -> usize {
        self.max_chunk_bytes.get()
    }

    pub const fn max_concurrent_remote_uni_streams(self) -> u32 {
        self.max_concurrent_remote_uni_streams.get()
    }

    pub const fn max_concurrent_remote_bi_streams(self) -> u32 {
        self.max_concurrent_remote_bi_streams.get()
    }

    pub const fn stream_receive_window(self) -> u64 {
        self.stream_receive_window.get()
    }

    pub const fn connection_receive_window(self) -> u64 {
        self.connection_receive_window.get()
    }

    pub const fn idle_timeout(self) -> Duration {
        Duration::from_millis(self.idle_timeout_ms.get())
    }
}

impl Default for CandidateConfig {
    fn default() -> Self {
        Self {
            control_queue_capacity: nonzero_usize(DEFAULT_CONTROL_QUEUE_CAPACITY),
            incoming_stream_queue_capacity: nonzero_usize(DEFAULT_INCOMING_STREAM_QUEUE_CAPACITY),
            outgoing_stream_capacity: nonzero_usize(DEFAULT_OUTGOING_STREAM_CAPACITY),
            stream_chunk_queue_capacity: nonzero_usize(DEFAULT_STREAM_CHUNK_QUEUE_CAPACITY),
            max_control_frame_bytes: nonzero_usize(DEFAULT_MAX_CONTROL_FRAME_BYTES),
            max_opening_frame_bytes: nonzero_usize(DEFAULT_MAX_OPENING_FRAME_BYTES),
            max_chunk_bytes: nonzero_usize(DEFAULT_MAX_CHUNK_BYTES),
            max_concurrent_remote_uni_streams: nonzero_u32(
                DEFAULT_MAX_CONCURRENT_REMOTE_UNI_STREAMS,
            ),
            max_concurrent_remote_bi_streams: nonzero_u32(DEFAULT_MAX_CONCURRENT_REMOTE_BI_STREAMS),
            stream_receive_window: nonzero_u64(DEFAULT_STREAM_RECEIVE_WINDOW),
            connection_receive_window: nonzero_u64(DEFAULT_CONNECTION_RECEIVE_WINDOW),
            idle_timeout_ms: nonzero_u64(DEFAULT_IDLE_TIMEOUT_MS),
        }
    }
}

fn nonzero_usize(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).expect("candidate transport default must be nonzero")
}

fn nonzero_u32(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("candidate transport default must be nonzero")
}

fn nonzero_u64(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("candidate transport default must be nonzero")
}

use std::{
    future::Future,
    mem,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
};

use iroh::endpoint::{Connection, VarInt};
use tokio::{sync::watch, task::JoinHandle};

pub(crate) struct SharedState {
    terminal: AtomicBool,
    terminal_tx: watch::Sender<bool>,
}

impl SharedState {
    fn new() -> Self {
        let (terminal_tx, _) = watch::channel(false);
        Self {
            terminal: AtomicBool::new(false),
            terminal_tx,
        }
    }

    pub(crate) fn mark_terminal(&self) -> bool {
        if self.terminal.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.terminal_tx.send_replace(true);
        true
    }

    pub(crate) fn is_terminal(&self) -> bool {
        self.terminal.load(Ordering::Acquire)
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<bool> {
        self.terminal_tx.subscribe()
    }
}

struct TaskRegistryState {
    accepting: bool,
    handles: Vec<JoinHandle<()>>,
}

pub(crate) struct TaskRegistry {
    state: Mutex<TaskRegistryState>,
}

impl TaskRegistry {
    fn new() -> Self {
        Self {
            state: Mutex::new(TaskRegistryState {
                accepting: true,
                handles: Vec::new(),
            }),
        }
    }

    pub(crate) fn spawn<Fut>(&self, future: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut state = lock(&self.state);
        debug_assert!(state.accepting);
        state.handles.push(tokio::spawn(future));
    }

    pub(crate) fn close_and_take(&self) -> Vec<JoinHandle<()>> {
        let mut state = lock(&self.state);
        state.accepting = false;
        mem::take(&mut state.handles)
    }
}

pub(crate) struct CandidateRuntime {
    connection: Connection,
    shared: Arc<SharedState>,
    tasks: Arc<TaskRegistry>,
}

impl CandidateRuntime {
    pub(crate) fn new(connection: Connection) -> Self {
        Self {
            connection,
            shared: Arc::new(SharedState::new()),
            tasks: Arc::new(TaskRegistry::new()),
        }
    }

    pub(crate) fn connection(&self) -> Connection {
        self.connection.clone()
    }

    pub(crate) fn shared(&self) -> Arc<SharedState> {
        Arc::clone(&self.shared)
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<bool> {
        self.shared.subscribe()
    }

    pub(crate) fn spawn<Fut>(&self, future: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.tasks.spawn(future);
    }

    pub(crate) fn is_terminal(&self) -> bool {
        self.shared.is_terminal()
    }

    pub(crate) fn terminate(&self, code: VarInt, reason: &'static [u8]) {
        if self.shared.mark_terminal() {
            self.connection.close(code, reason);
        }
    }

    pub(crate) fn close(&self) {
        self.terminate(VarInt::from_u32(0), b"crosslab M9 control bridge closed");
    }

    pub(crate) async fn shutdown(self) {
        self.close();
        for task in self.tasks.close_and_take() {
            let _ = task.await;
        }
    }
}

pub(crate) fn terminate_shared(
    connection: &Connection,
    shared: &SharedState,
    code: VarInt,
    reason: &'static [u8],
) {
    if shared.mark_terminal() {
        connection.close(code, reason);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use super::TaskRegistry;

    #[tokio::test]
    async fn task_registry_rejects_spawn_after_shutdown_admission_closes() {
        let registry = TaskRegistry::new();
        assert!(registry.close_and_take().is_empty());

        let ran = Arc::new(AtomicBool::new(false));
        let ran_task = Arc::clone(&ran);
        assert!(!registry.spawn(async move {
            ran_task.store(true, Ordering::Release);
        }));
        tokio::task::yield_now().await;

        assert!(!ran.load(Ordering::Acquire));
    }
}
