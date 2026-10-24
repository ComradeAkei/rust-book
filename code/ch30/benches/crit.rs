//! Раздел 30.2: та же сумма квадратов под criterion.
//!
//! ```text
//! cargo bench -p ch30 --bench crit
//! ```

use ch30::pitfalls::sum_squares;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

// ANCHOR: crit
fn squares(c: &mut Criterion) {
    let mut group = c.benchmark_group("sum_squares");
    for n in [1_000u64, 10_000, 100_000] {
        let v: Vec<u64> = (0..n).collect();
        group.bench_with_input(BenchmarkId::from_parameter(n), &v, |b, v| {
            b.iter(|| sum_squares(black_box(v)))
        });
    }
    group.finish();
}

criterion_group!(benches, squares);
criterion_main!(benches);
// ANCHOR_END: crit
