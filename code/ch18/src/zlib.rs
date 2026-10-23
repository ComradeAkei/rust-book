//! Раздел 18.6: настоящая библиотека и инструменты. zlib — обычная системная
//! C-библиотека; объявления ниже — ровно то, что сгенерировал бы `bindgen` по
//! `zlib.h`, а поверх них написана безопасная обёртка. `#[link(name = "z")]`
//! называет библиотеку для компоновщика: по умолчанию Rust линкует только
//! libc, остальное — через `#[link]` или `cargo::rustc-link-lib` в build.rs.

use core::ffi::{c_uint, c_ulong};

// ANCHOR: bind
// Сырые объявления zlib (как от bindgen). #[link] просит компоновщик
// добавить -lz; без него символ crc32 не найдётся.
#[link(name = "z")]
unsafe extern "C" {
    fn crc32(crc: c_ulong, buf: *const u8, len: c_uint) -> c_ulong;
}

/// Контрольная сумма CRC-32 — безопасная обёртка над zlib. Срез даёт и
/// указатель, и длину; никаких предусловий наружу не торчит.
pub fn crc32_of(data: &[u8]) -> u32 {
    // SAFETY: crc32 читает ровно len байт по buf; срез гарантирует валидную
    // область этой длины, выравнивание u8 произвольно, память жива на вызов.
    (unsafe { crc32(0, data.as_ptr(), data.len() as c_uint) }) as u32
}
// ANCHOR_END: bind

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "вызывает C (zlib)")]
    fn crc32_matches_known_vector() {
        // Эталонный вектор zlib: CRC-32 строки "123456789".
        assert_eq!(crc32_of(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32_of(b""), 0);
    }
}
