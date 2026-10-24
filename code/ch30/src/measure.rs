//! Раздел 30.1: замер времени своими руками — прогрев, выборки, медиана
//! и разброс. Этого хватает, чтобы увидеть ошибки методики; для настоящих
//! сравнений — criterion и divan (раздел 30.2).

use std::hint::black_box;
use std::time::{Duration, Instant};

// ANCHOR: stats
/// Сводка выборок: время одного вызова, наносекунды.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub min: f64,
    pub median: f64,
    /// Медиана абсолютных отклонений от медианы: разброс, который не
    /// портят редкие выбросы — прерывание, вытеснение, промах TLB.
    pub mad: f64,
    pub max: f64,
}

fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 { sorted[n / 2] } else { (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0 }
}

impl Stats {
    /// Сводка непустого набора выборок; порядок выборок меняется.
    pub fn from_samples(ns: &mut [f64]) -> Stats {
        assert!(!ns.is_empty(), "нет выборок");
        ns.sort_by(f64::total_cmp);
        let med = median(ns);
        let mut dev: Vec<f64> = ns.iter().map(|x| (x - med).abs()).collect();
        dev.sort_by(f64::total_cmp);
        Stats { min: ns[0], median: med, mad: median(&dev), max: ns[ns.len() - 1] }
    }
}
// ANCHOR_END: stats

// ANCHOR: bench
/// `samples` выборок по `iters` вызовов `f`, перед ними — прогрев длиной
/// в одну выборку. Результат каждого вызова уходит в `black_box`, чтобы
/// оптимизатор не выбросил работу как ненужную.
pub fn bench<R>(samples: usize, iters: u32, mut f: impl FnMut() -> R) -> Stats {
    assert!(iters > 0);
    for _ in 0..iters {
        black_box(f());
    }
    let mut ns = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..iters {
            black_box(f());
        }
        ns.push(start.elapsed().as_nanos() as f64 / f64::from(iters));
    }
    Stats::from_samples(&mut ns)
}
// ANCHOR_END: bench

// ANCHOR: clock
/// Самый маленький ненулевой шаг между двумя соседними `Instant::now`
/// за `tries` попыток: меньше этого часы не различают.
pub fn clock_step(tries: u32) -> Duration {
    let mut best = Duration::MAX;
    for _ in 0..tries {
        let a = Instant::now();
        let mut b = Instant::now();
        while b == a {
            b = Instant::now();
        }
        best = best.min(b - a);
    }
    best
}

/// Цена одного `Instant::now`, наносекунды: столько замер добавляет
/// к каждой выборке.
pub fn clock_cost(calls: u32) -> f64 {
    let start = Instant::now();
    for _ in 0..calls {
        black_box(Instant::now());
    }
    start.elapsed().as_nanos() as f64 / f64::from(calls)
}
// ANCHOR_END: clock

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_of_known_samples() {
        let mut ns = [5.0, 1.0, 3.0, 100.0, 4.0];
        let s = Stats::from_samples(&mut ns);
        assert_eq!(s, Stats { min: 1.0, median: 4.0, mad: 1.0, max: 100.0 });
        let mut even = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(Stats::from_samples(&mut even).median, 2.5);
    }
}
