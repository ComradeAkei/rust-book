//! Задача 1.D. Слитное умножение-сложение против двух округлений.
//!
//! Функции `ch01d_separate` и `ch01d_fused` даны. Найдите входы, на которых
//! они расходятся, затем входы, на которых раздельный способ даёт ровно
//! ноль, а слитный нет. Объясните оба случая по битам мантиссы. Потом
//! реализуйте `product_error` одним вызовом FMA, без `f64` и других
//! широких типов, и снимите ассемблер обеих функций под три цели.
//!
//! ```text
//! cargo asm -p ch01-exercises --lib ch01d_fused
//! cargo asm -p ch01-exercises --lib --target-cpu=x86-64-v3 ch01d_fused
//! cargo +nightly asm -p ch01-exercises --lib --features nightly \
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

/// Входы, на которых `ch01d_separate` и `ch01d_fused` дают разный результат.
pub const DIFFER: (f32, f32, f32) = (0.0, 0.0, 0.0);

/// Входы, на которых `ch01d_separate` даёт ровно ноль, а `ch01d_fused` нет.
pub const CANCEL: (f32, f32, f32) = (0.0, 0.0, 0.0);

/// Округлённое произведение `p = a * b` и его точная ошибка `e`:
/// `a · b = p + e` без потерь (если нет переполнения и денормалов).
#[inline(never)]
pub fn product_error(a: f32, b: f32) -> (f32, f32) {
    todo!()
}
