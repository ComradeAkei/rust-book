//! Раздел 30.1: часы, debug против release и три ошибки методики.
//!
//! ```text
//! cargo run -p ch30 --bin timing
//! cargo run -p ch30 --release --bin timing
//! ```

use ch30::measure::{bench, clock_cost, clock_step};
use ch30::pitfalls::{sum_squares, time_constant, time_discarded, time_kept, time_opaque};

fn per_call(d: std::time::Duration, iters: u32) -> f64 {
    d.as_nanos() as f64 / f64::from(iters)
}

fn main() {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    println!("сборка: {profile}");
    println!(
        "шаг часов: {:?}, цена Instant::now: {:.1} нс",
        clock_step(1000),
        clock_cost(1_000_000)
    );

    let v: Vec<u64> = (0..10_000).collect();
    let s = bench(31, 200, || sum_squares(&v));
    println!(
        "sum_squares, 10 000 чисел: медиана {:.0} нс (мин {:.0}, MAD {:.0}), {:.3} нс на число",
        s.median,
        s.min,
        s.mad,
        s.median / 10_000.0
    );

    let iters = 10_000;
    println!(
        "результат выброшен:   {:8.1} нс на вызов",
        per_call(time_discarded(&v, iters), iters)
    );
    println!("результат сохранён:   {:8.1} нс на вызов", per_call(time_kept(&v, iters), iters));
    // В debug цикл в sum_to настоящий: миллиард сложений на вызов — это
    // секунды, поэтому вызовов мало, а самый длинный ряд пропущен.
    let (iters, ns): (u32, &[u64]) = if cfg!(debug_assertions) {
        (5, &[1_000, 1_000_000])
    } else {
        (1_000_000, &[1_000, 1_000_000, 1_000_000_000])
    };
    println!("sum_to(1 000 000):    {:8.2} нс на вызов", per_call(time_constant(iters), iters));
    for &n in ns {
        println!(
            "sum_to(black_box({n})): {:8.2} нс на вызов",
            per_call(time_opaque(n, iters), iters)
        );
    }
}
