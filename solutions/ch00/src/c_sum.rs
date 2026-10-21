//! Задача 0.C: три суммы и их ассемблер на x86_64 и Cortex-M4F.

// ANCHOR: wrapping
/// Сумма по модулю 2³². `wrapping_add` разрешает компилятору переставлять
/// сложения, поэтому цикл векторизуется.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch00c_sum_wrapping(xs: &[u32]) -> u32 {
    xs.iter().fold(0u32, |acc, &x| acc.wrapping_add(x))
}
// ANCHOR_END: wrapping

// ANCHOR: wide
/// Точная сумма. Переполнения нет: даже 2³² элементов по `u32::MAX` дают
/// меньше 2⁶⁴.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch00c_sum_wide(xs: &[u32]) -> u64 {
    xs.iter().map(|&x| x as u64).sum()
}
// ANCHOR_END: wide

// ANCHOR: checked_naive
/// Первая версия: проверка после каждого сложения. Правильно, но цикл не
/// векторизуется, потому что выход из него зависит от каждого элемента.
pub fn sum_checked_naive(xs: &[u32]) -> Option<u32> {
    xs.iter().try_fold(0u32, |acc, &x| acc.checked_add(x))
}
// ANCHOR_END: checked_naive

// ANCHOR: checked
/// Вторая версия: считаем точно в `u64` и проверяем результат один раз на
/// кусок. Сумма куска из 2³¹ элементов меньше 2⁶³, а промежуточный итог
/// перед проверкой не больше `u32::MAX` плюс сумма куска, так что `u64` не
/// переполняется ни на какой длине среза.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch00c_sum_checked(xs: &[u32]) -> Option<u32> {
    const CHUNK: usize = 1 << 31;
    let mut total: u64 = 0;
    for chunk in xs.chunks(CHUNK) {
        total += chunk.iter().map(|&x| x as u64).sum::<u64>();
        if total > u32::MAX as u64 {
            return None;
        }
    }
    Some(total as u32)
}
// ANCHOR_END: checked

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_and_chunked_agree() {
        let cases: [&[u32]; 5] =
            [&[], &[7], &[u32::MAX, 1], &[u32::MAX - 1, 1], &[1 << 31, 1 << 30, 1 << 30]];
        for xs in cases {
            assert_eq!(sum_checked_naive(xs), ch00c_sum_checked(xs), "{xs:?}");
        }
    }
}
