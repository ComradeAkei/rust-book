//! Раздел 1.3: битовые методы целых и инструкции, в которые они
//! превращаются.
//!
//! ```text
//! cargo asm -p ch01 --lib ch01_clz
//! cargo asm -p ch01 --lib --target thumbv7m-none-eabi ch01_clz
//! cargo asm -p ch01 --lib --target thumbv6m-none-eabi ch01_clz
//! ```

// ANCHOR: bits
/// Число ведущих нулей: `lzcnt` или `bsr` на x86_64, `clz` на ARMv7-M.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_clz(x: u32) -> u32 {
    x.leading_zeros()
}

/// Число единичных битов: `popcnt`, если процессор его умеет.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_popcount(x: u32) -> u32 {
    x.count_ones()
}

/// Смена порядка байт: `bswap` на x86_64, `rev` на ARM.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_swap_bytes(x: u32) -> u32 {
    x.swap_bytes()
}

/// Циклический сдвиг: `rol` на x86_64, `ror` на ARM (влево на n — то же,
/// что вправо на 32 − n).
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_rotl(x: u32, n: u32) -> u32 {
    x.rotate_left(n)
}
// ANCHOR_END: bits

// ANCHOR: log2
/// Номер старшего единичного бита, то есть ⌊log₂ x⌋. Для нуля ответа нет,
/// и метод `ilog2` в этом случае паникует, а `checked_ilog2` даёт `None`.
pub fn floor_log2(x: u32) -> Option<u32> {
    x.checked_ilog2()
}
// ANCHOR_END: log2

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_methods() {
        assert_eq!(ch01_clz(0), 32);
        assert_eq!(ch01_clz(1), 31);
        assert_eq!(ch01_clz(0x8000_0000), 0);
        assert_eq!(ch01_popcount(0xF0F0), 8);
        assert_eq!(ch01_swap_bytes(0x1122_3344), 0x4433_2211);
        assert_eq!(ch01_rotl(0x8000_0001, 1), 3);
        assert_eq!(ch01_rotl(1, 33), 2);
        assert_eq!(floor_log2(0), None);
        assert_eq!(floor_log2(1), Some(0));
        assert_eq!(floor_log2(1000), Some(9));
    }
}
