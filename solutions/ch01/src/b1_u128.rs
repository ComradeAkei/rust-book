//! Задача 1.B1. 128-битная арифметика на парах `u64` с переносом.
//!
//! ```text
//! cargo test -p ch01-solutions task_b1
//! cargo asm -p ch01-solutions --lib ch01b_add
//! cargo asm -p ch01-solutions --lib --target thumbv7em-none-eabihf ch01b_add
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

// ANCHOR: add
/// Сумма по модулю 2¹²⁸ и признак переноса из старшего разряда.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01b_add(a: U128, b: U128) -> (U128, bool) {
    let (lo, c0) = a.lo.overflowing_add(b.lo);
    let (hi, c1) = a.hi.overflowing_add(b.hi);
    let (hi, c2) = hi.overflowing_add(u64::from(c0));
    // Оба переноса сразу не случаются: если a.hi + b.hi переполнилось, то
    // hi не больше 2⁶⁴ − 2 и прибавление единицы уже не переполнит.
    (U128 { lo, hi }, c1 | c2)
}
// ANCHOR_END: add

// ANCHOR: mul_wide
/// Полное произведение 64 × 64 → 128 бит школьным умножением на 32-битных
/// половинах: четыре частичных произведения, каждое помещается в `u64`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01b_mul_wide(a: u64, b: u64) -> U128 {
    const LOW: u64 = 0xFFFF_FFFF;
    let (a0, a1) = (a & LOW, a >> 32);
    let (b0, b1) = (b & LOW, b >> 32);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    // Средний столбец: старшая половина p00 и младшие половины p01 и p10.
    // Каждое слагаемое меньше 2³², сумма меньше 3 · 2³² и в u64 помещается.
    let mid = (p00 >> 32) + (p01 & LOW) + (p10 & LOW);
    let lo = mid << 32 | p00 & LOW;
    // Точное произведение меньше 2¹²⁸, поэтому старшая половина тоже не
    // переполняется, хотя проверить это сложением по частям нельзя.
    let hi = p11 + (p01 >> 32) + (p10 >> 32) + (mid >> 32);
    U128 { lo, hi }
}
// ANCHOR_END: mul_wide

// ANCHOR: mul
/// Произведение по модулю 2¹²⁸.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01b_mul(a: U128, b: U128) -> U128 {
    let low = ch01b_mul_wide(a.lo, b.lo);
    // Перекрёстные произведения сдвинуты на 64 бита: в результат попадают
    // только их младшие половины, а a.hi · b.hi целиком уходит за 2¹²⁸.
    let cross = a.lo.wrapping_mul(b.hi).wrapping_add(a.hi.wrapping_mul(b.lo));
    U128 { lo: low.lo, hi: low.hi.wrapping_add(cross) }
}
// ANCHOR_END: mul

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
