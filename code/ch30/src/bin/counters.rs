//! Раздел 30.3: какие счётчики даёт эта машина.
//!
//! ```text
//! cargo run -p ch30 --release --bin counters
//! ```

use ch30::perf_event::{Counter, Event, touch_fresh_pages};
use ch30::pitfalls::sum_squares;
use std::hint::black_box;

// ANCHOR: main
fn main() {
    let v: Vec<u64> = (0..1_000_000).collect();
    for event in Event::ALL {
        let counter = match Counter::new(event) {
            Ok(c) => c,
            Err(e) => {
                println!("{:>16}: не открыт: {e}", event.name());
                continue;
            }
        };
        let work = || match event {
            Event::PageFaults => touch_fresh_pages(1000).unwrap(),
            _ => sum_squares(black_box(&v)),
        };
        let (r, n) = counter.count(work).unwrap();
        black_box(r);
        println!("{:>16}: {n}", event.name());
    }
}
// ANCHOR_END: main
