//! Раздел 1.1: целые фиксированной ширины, их размер, выравнивание и
//! порядок байт в памяти.

// ANCHOR: bytes
/// Байты числа в памяти. На x86_64 и Cortex-M порядок little-endian:
/// младший байт по младшему адресу. `to_le_bytes` и `to_be_bytes` дают
/// одинаковый результат на любой платформе, `to_ne_bytes` повторяет память.
pub fn bytes_in_memory(x: u32) -> [u8; 4] {
    x.to_ne_bytes()
}
// ANCHOR_END: bytes

// ANCHOR: usize
/// `usize` — ширина адреса. На x86_64 это 64 бита, на Cortex-M 32.
pub const POINTER_BITS: u32 = usize::BITS;

/// Сколько байт можно адресовать срезом: длина среза не больше `isize::MAX`.
pub const MAX_SLICE_BYTES: usize = isize::MAX as usize;
// ANCHOR_END: usize

// ANCHOR: arm_align
// На Cortex-M по AAPCS 64- и 128-битные целые выравниваются на 8, а адрес
// занимает 32 бита. Проверка выполняется при сборке под любую цель ARM.
#[cfg(target_arch = "arm")]
const _: () = {
    assert!(core::mem::align_of::<u64>() == 8);
    assert!(core::mem::align_of::<u128>() == 8);
    assert!(usize::BITS == 32);
};
// ANCHOR_END: arm_align

// ANCHOR: promote
/// Сумма двух байт. В C операнды `uint8_t` сначала превращаются в `int`, и
/// сравнение `a + b > 255` честно даёт истину. В Rust типы операндов и
/// результата совпадают: `a + b` — это `u8`, и на 200 + 100 сложение
/// переполняется. Чтобы получить ответ как в C, расширять нужно явно.
/// Смешать ширины без расширения нельзя вовсе:
///
/// ```compile_fail,E0308
/// let (a, b): (u8, u16) = (200, 100);
/// let _ = a + b;
/// ```
pub fn sum_exceeds_byte(a: u8, b: u8) -> bool {
    u16::from(a) + u16::from(b) > 255
}
// ANCHOR_END: promote

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    // Размеры целых гарантированы языком на любой платформе.
    #[test]
    fn sizes_are_fixed() {
        assert_eq!(size_of::<u8>(), 1);
        assert_eq!(size_of::<i16>(), 2);
        assert_eq!(size_of::<u32>(), 4);
        assert_eq!(size_of::<i64>(), 8);
        assert_eq!(size_of::<u128>(), 16);
        assert_eq!(size_of::<usize>(), size_of::<*const u8>());
    }

    // Выравнивание u128 и u64 — свойство платформы, а не языка.
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn align_on_x86_64() {
        assert_eq!(align_of::<u64>(), 8);
        assert_eq!(align_of::<u128>(), 16);
        assert_eq!(POINTER_BITS, 64);
    }

    #[test]
    fn little_endian_layout() {
        if cfg!(target_endian = "little") {
            assert_eq!(bytes_in_memory(0x1122_3344), [0x44, 0x33, 0x22, 0x11]);
        }
        assert_eq!(0x1122_3344u32.to_be_bytes(), [0x11, 0x22, 0x33, 0x44]);
        assert_eq!(u32::from_le_bytes([0x44, 0x33, 0x22, 0x11]), 0x1122_3344);
    }

    #[test]
    fn widening_gives_c_answer() {
        assert!(sum_exceeds_byte(200, 100));
        assert!(!sum_exceeds_byte(200, 55));
    }
}
