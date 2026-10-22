//! Раздел 0.5. Каждая функция содержит ошибку, а rustc замечает только
//! одну из трёх, и то как предупреждение. Запуск:
//! `cd code/ch00/lints && cargo clippy`.

// ANCHOR: lints
/// Копирует кадр в буфер передачи.
pub fn copy_frame(dst: &mut [u8], src: &[u8]) {
    for i in 0..src.len() {
        dst[i] = src[i];
    }
}

/// Читает регистр периферии.
pub fn read_reg(addr: *const u32) -> u32 {
    unsafe { addr.read_volatile() }
}

/// Уровень сигнала в допустимых пределах?
pub fn level_ok(level: u8) -> bool {
    level <= 255
}
// ANCHOR_END: lints
