#![no_main]

use crosslab_protocol::decode_pairing_bootstrap;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_pairing_bootstrap(data);
});
