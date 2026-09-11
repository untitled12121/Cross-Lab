#![no_main]

use crosslab_policy::{CapabilityId, OperationName};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = core::str::from_utf8(data) {
        let _ = CapabilityId::parse(value);
        let _ = OperationName::parse(value);
    }
});
