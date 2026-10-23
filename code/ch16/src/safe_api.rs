//! Раздел 16.6: маленький `unsafe` за безопасным API. Приём всей главы:
//! запереть необходимый `unsafe` в короткой функции, чья сигнатура
//! безопасна, а предусловия проверены внутри. Здесь — выдать две
//! непересекающиеся изменяемые строки матрицы, чего заимствование само по
//! одному индексу не позволяет.

// ANCHOR: split
/// Две разные строки матрицы `rows × cols` (хранится построчно) как
/// непересекающиеся `&mut [T]`. Безопасный код не даст взять две `&mut` в
/// один `Vec` по индексам, но если строки действительно разные, срезы не
/// пересекаются — и это можно доказать.
pub fn two_rows_mut<T>(m: &mut [T], cols: usize, a: usize, b: usize) -> (&mut [T], &mut [T]) {
    assert!(a != b, "строки должны быть разными");
    assert!(cols > 0 && (a.max(b) + 1) * cols <= m.len(), "строка за границами");
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    // Режем между строками: левая часть содержит строку lo, правая — hi.
    let (left, right) = m.split_at_mut(hi * cols);
    let row_lo = &mut left[lo * cols..lo * cols + cols];
    let row_hi = &mut right[..cols];
    if a < b { (row_lo, row_hi) } else { (row_hi, row_lo) }
}
// ANCHOR_END: split

// ANCHOR: split_raw
/// То же через сырые указатели, когда `split_at_mut` не подходит (например,
/// строки нужно вернуть в исходном порядке без перестановки). `unsafe`
/// заперт в трёх строках, предусловие проверено `assert`-ами выше.
pub fn two_rows_raw<T>(m: &mut [T], cols: usize, a: usize, b: usize) -> (&mut [T], &mut [T]) {
    assert!(a != b, "строки должны быть разными");
    assert!(cols > 0 && (a.max(b) + 1) * cols <= m.len(), "строка за границами");
    let base: *mut T = m.as_mut_ptr();
    // SAFETY: a != b, поэтому отрезки [a*cols, a*cols+cols) и
    // [b*cols, …) не пересекаются; оба в границах по assert; base жив на
    // время заёма m. Две &mut на непересекающуюся память — не алиасинг.
    unsafe {
        let ra = core::slice::from_raw_parts_mut(base.add(a * cols), cols);
        let rb = core::slice::from_raw_parts_mut(base.add(b * cols), cols);
        (ra, rb)
    }
}
// ANCHOR_END: split_raw

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_disjoint() {
        let mut m = vec![0u32; 9]; // 3×3
        let (r0, r2) = two_rows_mut(&mut m, 3, 0, 2);
        r0.copy_from_slice(&[1, 2, 3]);
        r2.copy_from_slice(&[7, 8, 9]);
        assert_eq!(m, [1, 2, 3, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn raw_version_keeps_order() {
        let mut m = vec![0u32; 9];
        // Возвращает строки в порядке аргументов, даже когда b < a.
        let (r2, r0) = two_rows_raw(&mut m, 3, 2, 0);
        r2[0] = 70;
        r0[0] = 10;
        assert_eq!(m, [10, 0, 0, 0, 0, 0, 70, 0, 0]);
    }
}
