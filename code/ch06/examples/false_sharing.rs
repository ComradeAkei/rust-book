//! Ложное разделение кэш-линии: два потока увеличивают каждый свой счётчик.
//!
//! ```text
//! cargo run -p ch06 --release --example false_sharing
//! ```
//! Логически потоки не делят данных, но если счётчики лежат в одной
//! кэш-линии, ядра всё время отбирают её друг у друга по протоколу MESI.

use ch06::cache::{Counters, PaddedCounters};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const ITERS: u64 = 50_000_000;

// ANCHOR: bench
fn hammer(a: &AtomicU64, b: &AtomicU64) -> Duration {
    let start = Instant::now();
    thread::scope(|s| {
        s.spawn(|| {
            for _ in 0..ITERS {
                black_box(a).fetch_add(1, Ordering::Relaxed);
            }
        });
        s.spawn(|| {
            for _ in 0..ITERS {
                black_box(b).fetch_add(1, Ordering::Relaxed);
            }
        });
    });
    start.elapsed()
}
// ANCHOR_END: bench

fn main() {
    let cpus = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    if cpus < 2 {
        println!("нужно минимум два ядра, доступно {cpus}");
        return;
    }
    for round in 1..=3 {
        let near = Counters::default();
        let far = PaddedCounters::default();
        let t_near = hammer(&near.produced, &near.consumed);
        let t_far = hammer(&far.produced, &far.consumed);
        println!(
            "прогон {round}: одна линия {:>7.1} мс, разные линии {:>7.1} мс, отношение {:.1}",
            t_near.as_secs_f64() * 1e3,
            t_far.as_secs_f64() * 1e3,
            t_near.as_secs_f64() / t_far.as_secs_f64()
        );
    }
}
