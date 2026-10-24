//! Раздел 30.6: один и тот же отбор на случайных и на отсортированных
//! данных.
//!
//! ```text
//! cargo run -p ch30 --release --bin branches
//! valgrind --tool=cachegrind --branch-sim=yes target/release/branches random branchy
//! ```

use ch30::branches::{count_big, select_branchless, select_branchy};
use ch30::hot::noise;
use ch30::measure::bench;
use std::hint::black_box;

// ANCHOR: main
fn main() {
    let mut args = std::env::args().skip(1);
    let order = args.next();
    let only = args.next();
    let n = 1 << 22;
    let random = noise(n, 7);
    let mut sorted = random.clone();
    sorted.sort_unstable();
    let mut out = Vec::with_capacity(n);
    for (name, data) in [("random", &random), ("sorted", &sorted)] {
        if order.as_deref().is_some_and(|o| o != name) {
            continue;
        }
        type Select = fn(&[u8], &mut Vec<u8>) -> usize;
        let variants: [(&str, Select); 2] =
            [("branchy", select_branchy), ("branchless", select_branchless)];
        for (v, f) in variants {
            if only.as_deref().is_some_and(|o| o != v) {
                continue;
            }
            let s = bench(7, 1, || f(black_box(data), &mut out));
            println!("{name} {v:>10}: {:.3} нс на байт", s.median / n as f64);
        }
        if only.is_none() {
            let s = bench(7, 1, || count_big(black_box(data)));
            println!("{name} {:>10}: {:.3} нс на байт", "count", s.median / n as f64);
        }
    }
}
// ANCHOR_END: main
