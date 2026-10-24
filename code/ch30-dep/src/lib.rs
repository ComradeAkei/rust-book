//! Глава 30, раздел 30.5: функции «чужого» крейта.

// ANCHOR: dep
/// Маленькая функция-лист: в ней нет вызовов.
pub fn scale(x: u32) -> u32 {
    x.wrapping_mul(3).wrapping_add(1)
}

/// Не лист: внутри вызовы итератора и `scale`.
pub fn scaled_sum(v: &[u32]) -> u32 {
    v.iter().map(|&x| scale(x)).fold(0, u32::wrapping_add)
}

/// То же тело с `#[inline]`.
#[inline]
pub fn scaled_sum_inline(v: &[u32]) -> u32 {
    v.iter().map(|&x| scale(x)).fold(0, u32::wrapping_add)
}

/// То же тело, обобщённое по типу элемента.
pub fn scaled_sum_generic<T: Copy + Into<u32>>(v: &[T]) -> u32 {
    v.iter().map(|&x| scale(x.into())).fold(0, u32::wrapping_add)
}
// ANCHOR_END: dep
