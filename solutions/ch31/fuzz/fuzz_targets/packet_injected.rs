// ANCHOR: all
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ch31_solutions::b_fuzz::injected::check_packet(data));
// ANCHOR_END: all
