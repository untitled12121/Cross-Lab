#![no_main]

use crosslab_protocol::decode_control_envelope;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_control_envelope(data);
});
