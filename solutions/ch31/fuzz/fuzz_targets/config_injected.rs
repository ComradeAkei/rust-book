// ANCHOR: all
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Токенизатор принимает только текст: остальные входы пропускаем.
    if let Ok(text) = std::str::from_utf8(data) {
        ch31_solutions::b_fuzz::injected::check_config(text);
    }
});
// ANCHOR_END: all
