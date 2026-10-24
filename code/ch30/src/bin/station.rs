//! Раздел 30.9: сколько стоит приём кадра станцией — хранилище проекта и
//! два ускорения.
//!
//! ```text
//! cargo run -p ch30 --release --bin station
//! cargo run -p ch30 --release --bin station -- 1048576 x2
//! ```

use ch30::measure::bench;
use ch30::station::{HashStore, LoopStore, X2Store, stream};
use telemetry_store::Store;

// ANCHOR: main
fn main() {
    let mut args = std::env::args().skip(1);
    let frames: usize = args.next().map_or(1 << 20, |s| s.parse().unwrap());
    let only = args.next();
    let bytes = stream(frames, 64);
    println!("{frames} кадров, {} МиБ, 64 датчика", bytes.len() >> 20);
    let run = |name: &str, ingest: &mut dyn FnMut() -> usize| {
        if only.as_deref().is_some_and(|o| o != name) {
            return;
        }
        let s = bench(15, 1, ingest);
        println!(
            "{name:>5}: медиана {:5.1} нс на кадр (мин {:5.1}, MAD {:.1})",
            s.median / frames as f64,
            s.min / frames as f64,
            s.mad / frames as f64
        );
    };
    let mut store = Store::new(1024);
    run("store", &mut || store.ingest(&bytes).stored);
    let mut lp = LoopStore::new(1024);
    run("loop", &mut || lp.ingest(&bytes).stored);
    let mut hash = HashStore::new(1024);
    run("hash", &mut || hash.ingest(&bytes).stored);
    let mut x2 = X2Store::new(1024);
    run("x2", &mut || x2.ingest(&bytes).stored);
}
// ANCHOR_END: main
