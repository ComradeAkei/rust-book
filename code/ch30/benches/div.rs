//! Раздел 30.2: та же сумма квадратов под divan.
//!
//! ```text
//! cargo bench -p ch30 --bench div
//! ```

use ch30::pitfalls::sum_squares;
use divan::{Bencher, black_box};

// ANCHOR: div
fn main() {
    divan::main();
}

#[divan::bench(args = [1_000, 10_000, 100_000])]
fn squares(bencher: Bencher, n: u64) {
    let v: Vec<u64> = (0..n).collect();
    bencher.bench(|| sum_squares(black_box(&v)));
}
// ANCHOR_END: div
