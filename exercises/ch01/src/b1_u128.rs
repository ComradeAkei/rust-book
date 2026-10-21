//! Задача 1.B1. 128-битная арифметика на парах `u64` с переносом.
//!
//! Реализуйте три функции, не используя `u128` (кроме готовых
//! преобразований для тестов) и нестабильные `carrying_*`/`widening_*`.
//! Затем сравните ассемблер своих функций с эталонами на встроенном `u128`
//! в конце файла на x86_64 и на Cortex-M4F.
//!
//! ```text
//! cargo test -p ch01-exercises task_b1
//! cargo asm -p ch01-exercises --lib ch01b_add
//! cargo asm -p ch01-exercises --lib --target thumbv7em-none-eabihf ch01b_add
//! ```

/// Беззнаковое 128-битное число из двух 64-битных половин.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct U128 {
    pub lo: u64,
    pub hi: u64,
}

impl U128 {
    /// Только для тестов и сравнения: арифметика ниже обходится без `u128`.
    pub const fn from_u128(x: u128) -> Self {
        Self { lo: x as u64, hi: (x >> 64) as u64 }
    }

    pub const fn to_u128(self) -> u128 {
        (self.hi as u128) << 64 | self.lo as u128
    }
}

/// Сумма по модулю 2¹²⁸ и признак переноса из старшего разряда.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01b_add(a: U128, b: U128) -> (U128, bool) {
    todo!()
}

/// Полное произведение 64 × 64 → 128 бит школьным умножением на 32-битных
/// половинах: четыре частичных произведения, каждое помещается в `u64`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01b_mul_wide(a: u64, b: u64) -> U128 {
    todo!()
}

/// Произведение по модулю 2¹²⁸.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01b_mul(a: U128, b: U128) -> U128 {
    todo!()
}

// Эталоны на встроенном u128: с ними сравнивается ассемблер.

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01b_add_native(a: u128, b: u128) -> (u128, bool) {
    a.overflowing_add(b)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01b_mul_wide_native(a: u64, b: u64) -> u128 {
    u128::from(a) * u128::from(b)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01b_mul_native(a: u128, b: u128) -> u128 {
    a.wrapping_mul(b)
}
