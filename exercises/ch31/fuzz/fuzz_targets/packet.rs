#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ch31_exercises::b_fuzz::check_packet(data));
