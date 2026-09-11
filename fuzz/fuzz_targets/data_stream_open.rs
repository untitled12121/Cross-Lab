#![no_main]

use crosslab_protocol::decode_data_stream_open;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_data_stream_open(data);
});
