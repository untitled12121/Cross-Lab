use std::{num::NonZeroUsize, time::Duration};

const TEST_BULK_BYTES: usize = 64 * 1024;
const DEFAULT_BULK_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvalConfig {
    samples: NonZeroUsize,
    bulk_payload_bytes: NonZeroUsize,
    timeout: Duration,
}

impl EvalConfig {
    pub const fn test() -> Self {
        Self {
            samples: nonzero(1),
            bulk_payload_bytes: nonzero(TEST_BULK_BYTES),
            timeout: Duration::from_secs(5),
        }
    }

    pub const fn samples(self) -> usize {
        self.samples.get()
    }

    pub const fn bulk_payload_bytes(self) -> usize {
        self.bulk_payload_bytes.get()
    }

    pub const fn timeout(self) -> Duration {
        self.timeout
    }
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            samples: nonzero(5),
            bulk_payload_bytes: nonzero(DEFAULT_BULK_BYTES),
            timeout: Duration::from_secs(15),
        }
    }
}

const fn nonzero(value: usize) -> NonZeroUsize {
    match NonZeroUsize::new(value) {
        Some(value) => value,
        None => panic!("M9 evaluation defaults must be nonzero"),
    }
}
