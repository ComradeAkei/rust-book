//! Раздел 25.7, сквозной проект: CRC-16 кадра по таблице, посчитанной при
//! компиляции, против побитового цикла. Обе функции крейта
//! `telemetry-frame` — `const fn`; здесь они обёрнуты в функции с
//! неискажёнными именами, чтобы их тела было легко найти в ассемблере
//! (тест `tests/asm.rs`).

use telemetry_frame::{crc16_update, crc16_update_table};

// ANCHOR: wrappers
/// CRC кадра побитово: восемь сдвигов на байт.
///
/// # Safety
///
/// `data` указывает на `len` байт, которые можно читать.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ch25_crc_bitwise(data: *const u8, len: usize) -> u16 {
    // SAFETY: по контракту функции `data` и `len` описывают один буфер.
    crc16_update(0xFFFF, unsafe { core::slice::from_raw_parts(data, len) })
}

/// То же по таблице `CRC16_TABLE`: один поиск на байт.
///
/// # Safety
///
/// Как у [`ch25_crc_bitwise`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ch25_crc_table(data: *const u8, len: usize) -> u16 {
    // SAFETY: как выше.
    crc16_update_table(0xFFFF, unsafe { core::slice::from_raw_parts(data, len) })
}
// ANCHOR_END: wrappers

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_crc() {
        let frame = crate::tables::MEASUREMENT_FRAME;
        // SAFETY: указатели и длины взяты у живых массивов.
        let (a, b, check) = unsafe {
            (
                ch25_crc_bitwise(frame.as_ptr(), 6),
                ch25_crc_table(frame.as_ptr(), 6),
                ch25_crc_table(b"123456789".as_ptr(), 9),
            )
        };
        assert_eq!(a, b);
        assert_eq!(check, 0x29B1);
    }
}
