//! Раздел 30.4: один и тот же обход матрицы — по строкам и по столбцам.

// ANCHOR: matrix
/// Квадратная матрица `n × n` по строкам: элемент (i, j) — `m[i * n + j]`.
pub fn matrix(n: usize) -> Vec<f64> {
    (0..n * n).map(|k| (k % 7) as f64).collect()
}

/// Сумма по строкам: адреса подряд, одна строка кэша — восемь чисел.
#[inline(never)]
pub fn sum_rows(m: &[f64], n: usize) -> f64 {
    let mut s = 0.0;
    for i in 0..n {
        for j in 0..n {
            s += m[i * n + j];
        }
    }
    s
}

/// Сумма по столбцам: те же числа, шаг между соседними — `8 * n` байт.
#[inline(never)]
pub fn sum_cols(m: &[f64], n: usize) -> f64 {
    let mut s = 0.0;
    for j in 0..n {
        for i in 0..n {
            s += m[i * n + j];
        }
    }
    s
}
// ANCHOR_END: matrix

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_sum() {
        let m = matrix(64);
        assert_eq!(sum_rows(&m, 64), sum_cols(&m, 64));
    }
}
