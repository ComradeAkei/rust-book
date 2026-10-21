//! Задача 1.D. Слитное умножение-сложение против двух округлений.
//!
//! ```text
//! cargo asm -p ch01-solutions --lib ch01d_fused
//! cargo asm -p ch01-solutions --lib --target-cpu=x86-64-v3 ch01d_fused
//! cargo +nightly asm -p ch01-solutions --lib --features nightly \
//!     --target thumbv7em-none-eabihf ch01d_fused
//! ```

/// Два округления: сначала произведение, потом сумма.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01d_separate(a: f32, b: f32, c: f32) -> f32 {
    a * b + c
}

/// Одно округление: `a · b + c` считается точно и округляется один раз.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01d_fused(a: f32, b: f32, c: f32) -> f32 {
    fma(a, b, c)
}

/// `mul_add` из std, а на цели без ОС из core: там он пока только на nightly.
#[inline(always)]
fn fma(a: f32, b: f32, c: f32) -> f32 {
    #[cfg(not(target_os = "none"))]
    return a.mul_add(b, c);
    #[cfg(target_os = "none")]
    return core::f32::math::mul_add(a, b, c);
}

// 1 + 2⁻¹² точно представимо: дробная часть мантиссы 0x000800.
const A: f32 = f32::from_bits(0x3F80_0800);

// ANCHOR: witnesses
/// Расхождение в последнем бите. a² = 1 + 2⁻¹¹ + 2⁻²⁴, и раздельный способ
/// теряет 2⁻²⁴ при округлении произведения (ничья к чётному). С c = 2⁻²³
/// точная сумма равна 1 + 2⁻¹¹ + 1,5 · 2⁻²³: снова ничья, и FMA округляет
/// её вверх к чётной мантиссе, а раздельный способ получает нечётную.
pub const DIFFER: (f32, f32, f32) = (A, A, f32::EPSILON);

/// Раздельный способ даёт ровно ноль, слитный — потерянные 2⁻²⁴.
/// Здесь c = −(1 + 2⁻¹¹), то есть минус округлённое произведение.
pub const CANCEL: (f32, f32, f32) = (A, A, f32::from_bits(0xBF80_1000));
// ANCHOR_END: witnesses

// ANCHOR: two_product
/// Округлённое произведение и его точная ошибка: a · b = p + e без потерь.
#[inline(never)]
pub fn product_error(a: f32, b: f32) -> (f32, f32) {
    let p = a * b;
    // FMA считает a · b − p точно и округляет один раз. Точная разность
    // помещается в 24 бита (если нет переполнения и денормалов), так что
    // округлять нечего и e получается без ошибки.
    let e = fma(a, b, -p);
    (p, e)
}
// ANCHOR_END: two_product
