//! Раздел 16.3: что оптимизатор делает с UB и с его отсутствием. Индекс с
//! проверкой границ компилируется в сравнение и ветку на панику; тот же
//! доступ через `get_unchecked` — в одну загрузку. Разница — цена проверки,
//! которую `unsafe` убирает, переложив предусловие на программиста. А
//! обещание через `unreachable_unchecked` оптимизатор распространяет назад
//! и вырезает код до точки, где обещание нарушилось бы.

// ANCHOR: checked
/// Индекс с проверкой границ: при выходе за пределы — паника.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch16_checked(v: &[u32], i: usize) -> u32 {
    v[i]
}
// ANCHOR_END: checked

// ANCHOR: unchecked
/// Тот же доступ без проверки.
///
/// # Safety
///
/// `i < v.len()`.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn ch16_unchecked(v: &[u32], i: usize) -> u32 {
    // SAFETY: предусловие функции переложено на вызывающего.
    unsafe { *v.get_unchecked(i) }
}
// ANCHOR_END: unchecked

// ANCHOR: assume
/// Обещание: `n` чётно. Оптимизатор распространяет его назад — раз нечётное
/// `n` вело бы в `unreachable_unchecked` (UB), значит такого не бывает, и
/// `n / 2 * 2` заведомо равно `n`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch16_assume_even(n: u32) -> u32 {
    if n % 2 != 0 {
        // SAFETY: вызывающий обещал чётное n; нечётное сюда не попадает.
        unsafe { core::hint::unreachable_unchecked() }
    }
    n / 2 * 2
}
// ANCHOR_END: assume

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_and_unchecked_agree_in_bounds() {
        let v = [10u32, 20, 30];
        assert_eq!(ch16_checked(&v, 1), 20);
        // SAFETY: индекс 2 в границах среза длины 3.
        assert_eq!(unsafe { ch16_unchecked(&v, 2) }, 30);
        assert_eq!(ch16_assume_even(42), 42);
    }
}
