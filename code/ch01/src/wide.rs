//! Раздел 1.2: 128-битные целые на 64- и 32-битных процессорах.
//!
//! ```text
//! cargo asm -p ch01 --lib ch01_add_u128
//! cargo asm -p ch01 --lib --target thumbv7em-none-eabihf ch01_add_u128
//! ```

// ANCHOR: u128
/// Сложение u128: на x86_64 пара `add`/`adc`, на Cortex-M цепочка из
/// четырёх сложений с переносом.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_add_u128(a: u128, b: u128) -> u128 {
    a.wrapping_add(b)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_sub_u128(a: u128, b: u128) -> u128 {
    a.wrapping_sub(b)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_mul_u128(a: u128, b: u128) -> u128 {
    a.wrapping_mul(b)
}
// ANCHOR_END: u128

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carry_crosses_words() {
        assert_eq!(ch01_add_u128(u64::MAX as u128, 1), 1 << 64);
        assert_eq!(ch01_add_u128(u128::MAX, 1), 0);
        assert_eq!(ch01_sub_u128(1 << 64, 1), u64::MAX as u128);
        assert_eq!(
            ch01_mul_u128(u64::MAX as u128, u64::MAX as u128),
            0xFFFF_FFFF_FFFF_FFFE_0000_0000_0000_0001
        );
    }
}
