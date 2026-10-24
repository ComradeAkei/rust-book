// ANCHOR: all
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = ch31::cmd::parse_v1(data);
});
// ANCHOR_END: all
