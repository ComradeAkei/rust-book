//! Раздел 30.1: работа, которую меряем, и три способа измерить не её.

use std::hint::black_box;
use std::time::{Duration, Instant};

// ANCHOR: work
/// Сумма квадратов — работа, которую будем мерить.
pub fn sum_squares(v: &[u64]) -> u64 {
    v.iter().map(|x| x * x).sum()
}
// ANCHOR_END: work

// ANCHOR: sum_to
/// Сумма 0 + 1 + … + (n − 1) циклом.
pub fn sum_to(n: u64) -> u64 {
    (0..n).sum()
}
// ANCHOR_END: sum_to

// ANCHOR: discard
/// Ошибка методики: результат никуда не идёт.
pub fn time_discarded(v: &[u64], iters: u32) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        sum_squares(v);
    }
    start.elapsed()
}

/// То же, но вход и результат проходят через `black_box`.
pub fn time_kept(v: &[u64], iters: u32) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        black_box(sum_squares(black_box(v)));
    }
    start.elapsed()
}
// ANCHOR_END: discard

// ANCHOR: constant
/// Ошибка методики: аргумент известен при компиляции.
pub fn time_constant(iters: u32) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        black_box(sum_to(1_000_000));
    }
    start.elapsed()
}

/// Аргумент неизвестен оптимизатору — но и цикла уже нет.
pub fn time_opaque(n: u64, iters: u32) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        black_box(sum_to(black_box(n)));
    }
    start.elapsed()
}
// ANCHOR_END: constant

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums() {
        assert_eq!(sum_squares(&[1, 2, 3]), 14);
        assert_eq!(sum_to(0), 0);
        assert_eq!(sum_to(1_000_000), 499_999_500_000);
    }
}
