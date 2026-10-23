//! Задача C: скалярное произведение на итераторах — векторизация для `i32`
//! и её отсутствие для `f32`, и векторизация вручную через `chunks_exact`
//! и восемь сумм.

// ANCHOR: dot
/// Сумма произведений по модулю 2^32.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13c_dot_i32(a: &[i32], b: &[i32]) -> i32 {
    a.iter().zip(b).map(|(x, y)| x.wrapping_mul(*y)).fold(0, i32::wrapping_add)
}

/// Та же цепочка для `f32`: сложения идут строго по порядку элементов.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13c_dot_f32(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Восемь независимых сумм, по одной на дорожку вектора, и их сложение в
/// конце. Порядок сложений другой, и выбран он здесь, в исходном коде, а не
/// компилятором.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13c_dot_f32_lanes(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let (a, b) = (a[..n].chunks_exact(8), b[..n].chunks_exact(8));
    let tail: f32 = a.remainder().iter().zip(b.remainder()).map(|(x, y)| x * y).sum();
    let mut acc = [0.0f32; 8];
    for (x, y) in a.zip(b) {
        for ((s, p), q) in acc.iter_mut().zip(x).zip(y) {
            *s += p * q;
        }
    }
    acc.iter().sum::<f32>() + tail
}
// ANCHOR_END: dot
