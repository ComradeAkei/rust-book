//! Раздел 30.4: обход матрицы по строкам и по столбцам — для cachegrind
//! и для часов.
//!
//! ```text
//! cargo run -p ch30 --release --bin matrix -- rows 2048
//! valgrind --tool=cachegrind --cache-sim=yes target/release/matrix cols 2048
//! ```

use ch30::cache::{matrix, sum_cols, sum_rows};
use std::time::Instant;

// ANCHOR: main
fn main() {
    let mut args = std::env::args().skip(1);
    let order = args.next().expect("rows или cols");
    let n: usize = args.next().map_or(2048, |s| s.parse().unwrap());
    let m = matrix(n);
    let start = Instant::now();
    let s = match order.as_str() {
        "rows" => sum_rows(&m, n),
        "cols" => sum_cols(&m, n),
        _ => panic!("rows или cols"),
    };
    let ns = start.elapsed().as_nanos() as f64 / (n * n) as f64;
    println!("{order} {n}×{n}: сумма {s}, {ns:.2} нс на элемент");
}
// ANCHOR_END: main
