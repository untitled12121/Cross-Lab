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
pub struct QuicTransportConfig {
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

impl QuicTransportConfig {
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

impl Default for QuicTransportConfig {
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
            max_concurrent_remote_bi_streams: nonzero_u32(
                DEFAULT_MAX_CONCURRENT_REMOTE_BI_STREAMS,
            ),
            stream_receive_window: nonzero_u64(DEFAULT_STREAM_RECEIVE_WINDOW),
            connection_receive_window: nonzero_u64(DEFAULT_CONNECTION_RECEIVE_WINDOW),
            idle_timeout_ms: nonzero_u64(DEFAULT_IDLE_TIMEOUT_MS),
        }
    }
}

fn nonzero_usize(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).expect("QUIC transport default must be nonzero")
}

fn nonzero_u32(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("QUIC transport default must be nonzero")
}

fn nonzero_u64(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("QUIC transport default must be nonzero")
}
