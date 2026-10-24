//! Раздел 30.7: строки журнала через `format!` и через `write!` под
//! divan — с его счётчиком выделений вместо распределителя-счётчика
//! главы 15.
//!
//! ```text
//! cargo bench -p ch30 --bench alloc
//! ```

use ch30::memory::{lines_format, lines_write, readings};
use divan::{AllocProfiler, Bencher, black_box};

// ANCHOR: alloc
#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

#[divan::bench(args = [1_000])]
fn format(bencher: Bencher, n: usize) {
    let log = readings(n);
    bencher.bench(|| lines_format(black_box(&log)));
}

#[divan::bench(args = [1_000])]
fn write(bencher: Bencher, n: usize) {
    let log = readings(n);
    bencher.bench(|| lines_write(black_box(&log)));
}
// ANCHOR_END: alloc
