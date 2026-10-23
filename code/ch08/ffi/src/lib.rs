// ANCHOR: lib
/// Перевод сырого отсчёта в милли-единицы. Для Rust-пользователей.
pub fn scale(raw: u16, factor: u32) -> u32 {
    raw as u32 * factor
}

/// То же для C: имя без декорирования, соглашение о вызовах C.
#[unsafe(no_mangle)]
pub extern "C" fn sensor_scale(raw: u16, factor: u32) -> u32 {
    scale(raw, factor)
}
// ANCHOR_END: lib
