#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Токенизатор принимает только текст: остальные входы пропускаем.
    if let Ok(text) = std::str::from_utf8(data) {
        ch31_exercises::b_fuzz::check_config(text);
    }
});
