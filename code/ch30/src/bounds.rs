//! Раздел 30.6: проверки границ — где они остаются и как их убрать
//! без `unsafe`.

// ANCHOR: indexed
/// Сумма первых `n` элементов по индексу: проверка `i < v.len()` на
/// каждом шаге, и паника должна случиться ровно на том элементе, где
/// индекс вышел за границу.
// Цикл по индексу и есть предмет опыта, итератор clippy здесь не нужен.
#[allow(clippy::needless_range_loop)]
#[inline(never)]
pub fn sum_indexed(v: &[u32], n: usize) -> u32 {
    let mut s = 0u32;
    for i in 0..n {
        s = s.wrapping_add(v[i]);
    }
    s
}
// ANCHOR_END: indexed

// ANCHOR: sliced
/// То же, но граница проверена один раз до цикла: срез `&v[..n]` либо
/// паникует сразу, либо даёт срез длины ровно `n`.
// Цикл по индексу и есть предмет опыта, итератор clippy здесь не нужен.
#[allow(clippy::needless_range_loop)]
#[inline(never)]
pub fn sum_sliced(v: &[u32], n: usize) -> u32 {
    let v = &v[..n];
    let mut s = 0u32;
    for i in 0..n {
        s = s.wrapping_add(v[i]);
    }
    s
}

/// Итератор: проверять нечего.
#[inline(never)]
pub fn sum_iter(v: &[u32], n: usize) -> u32 {
    v[..n].iter().fold(0, |s, &x| s.wrapping_add(x))
}
// ANCHOR_END: sliced

// ANCHOR: unchecked
/// Без проверки вовсе. Ничем не быстрее `sum_sliced` — и неопределённое
/// поведение, если `n > v.len()`.
///
/// # Safety
///
/// `n <= v.len()`.
#[inline(never)]
pub unsafe fn sum_unchecked(v: &[u32], n: usize) -> u32 {
    let mut s = 0u32;
    for i in 0..n {
        // SAFETY: i < n <= v.len() по контракту функции.
        s = s.wrapping_add(unsafe { *v.get_unchecked(i) });
    }
    s
}
// ANCHOR_END: unchecked

// ANCHOR: hist
/// Гистограмма в чужой срез: индекс — байт данных, длина среза
/// неизвестна, проверка `b < h.len()` на каждом байте.
#[inline(never)]
pub fn hist_slice(data: &[u8], h: &mut [u32]) {
    for &b in data {
        h[usize::from(b)] += 1;
    }
}

/// Та же гистограмма в массив из 256 счётчиков: индекс из `u8` всегда
/// меньше 256, проверять нечего.
#[inline(never)]
pub fn hist_array(data: &[u8], h: &mut [u32; 256]) {
    for &b in data {
        h[usize::from(b)] += 1;
    }
}
// ANCHOR_END: hist

// ANCHOR: scale
/// Запись в выходной срез до чужого `n`: каждая запись видна снаружи,
/// поэтому паника обязана случиться ровно на первом плохом индексе, а
/// всё до него — записаться.
#[inline(never)]
pub fn scale_into(v: &[u32], out: &mut [u32], n: usize) {
    for i in 0..n {
        out[i] = v[i].wrapping_mul(3);
    }
}
// ANCHOR_END: scale

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_sums() {
        let v: Vec<u32> = (0..1000).collect();
        let want = sum_indexed(&v, 777);
        assert_eq!(sum_sliced(&v, 777), want);
        assert_eq!(sum_iter(&v, 777), want);
        // SAFETY: 777 <= 1000.
        assert_eq!(unsafe { sum_unchecked(&v, 777) }, want);
    }

    // ANCHOR: panics
    #[test]
    #[should_panic(expected = "index out of bounds: the len is 3 but the index is 3")]
    fn indexed_panics_at_the_element() {
        sum_indexed(&[1, 2, 3], 4);
    }

    #[test]
    #[should_panic(expected = "range end index 4 out of range for slice of length 3")]
    fn sliced_panics_before_the_loop() {
        sum_sliced(&[1, 2, 3], 4);
    }
    // ANCHOR_END: panics

    #[test]
    fn same_histograms() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i * 7919 % 251) as u8).collect();
        let (mut a, mut b) = (vec![0u32; 256], [0u32; 256]);
        hist_slice(&data, &mut a);
        hist_array(&data, &mut b);
        assert_eq!(a, b);
    }

    // ANCHOR: scale_test
    #[test]
    fn scale_writes_up_to_the_bad_index() {
        let v = [1, 2, 3, 4];
        let mut out = [0u32; 3];
        let r =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scale_into(&v, &mut out, 4)));
        assert!(r.is_err());
        assert_eq!(out, [3, 6, 9]);
    }
    // ANCHOR_END: scale_test
}
