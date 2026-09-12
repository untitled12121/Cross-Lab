#![no_main]

use crosslab_protocol::decode_session_auth_bootstrap;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_session_auth_bootstrap(data);
});
