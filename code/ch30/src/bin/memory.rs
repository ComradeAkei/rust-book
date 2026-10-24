//! Раздел 30.7: раскладка, ложное разделение и выделения в цикле.
//!
//! ```text
//! cargo run -p ch30 --release --bin memory
//! ```

use ch30::measure::bench;
use ch30::memory::{
    Adjacent, Columns, Packed, Padded, bump_pair, lines_format, lines_write, readings, sum_aos,
    sum_soa,
};
use std::hint::black_box;
use std::time::Instant;
use telemetry_store::{Counted, alloc::counts};

#[global_allocator]
static COUNTED: Counted = Counted;

// ANCHOR: main
fn main() {
    let only = std::env::args().nth(1);
    let want = |name: &str| only.as_deref().is_none_or(|o| o == name);
    let n = 1 << 22;
    let log = readings(n);
    if want("layout") {
        let cols = Columns::from_rows(&log);
        let aos = bench(11, 1, || sum_aos(black_box(&log)));
        let soa = bench(11, 1, || sum_soa(black_box(&cols)));
        println!(
            "AoS: {:.3} нс на запись, SoA: {:.3} нс",
            aos.median / n as f64,
            soa.median / n as f64
        );
    }
    if want("sharing") {
        let rounds = 20_000_000;
        let packed = Packed::default();
        let start = Instant::now();
        bump_pair(&packed.a, &packed.b, rounds);
        let near = start.elapsed();
        let adjacent = Adjacent::default();
        let start = Instant::now();
        bump_pair(&adjacent.a, &adjacent.b, rounds);
        let next = start.elapsed();
        let (a, b) = (Padded::default(), Padded::default());
        let start = Instant::now();
        bump_pair(&a.0, &b.0, rounds);
        let apart = start.elapsed();
        println!(
            "два счётчика в одной строке: {near:.0?}, в соседних: {next:.0?}, в разных: {apart:.0?}"
        );
    }
    if want("alloc") {
        let part = &log[..100_000];
        for (name, f) in [("format!", lines_format as fn(&_) -> String), ("write!", lines_write)] {
            let before = counts();
            let start = Instant::now();
            let s = black_box(f(part));
            let t = start.elapsed();
            let after = counts();
            println!(
                "{name:>8}: {:.0} нс на строку, выделений {}, {} байт",
                t.as_nanos() as f64 / part.len() as f64,
                after.allocs - before.allocs,
                s.len()
            );
        }
    }
}
// ANCHOR_END: main
