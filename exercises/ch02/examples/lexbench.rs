//! Задача 2.C: скорость конечного автомата и регулярных выражений на одной
//! и той же программе.
//!
//! ```console
//! $ cargo run -p ch02-exercises --release --example lexbench
//! ```

use ch02::lexer::generate;
use ch02_exercises::c_lexer::lex;
use ch02_exercises::c_regex::lex_regex;
use std::hint::black_box;
use std::time::{Duration, Instant};

/// Лучшее время из нескольких прогонов: меньше всего помех от системы.
fn best_of(runs: usize, mut f: impl FnMut()) -> Duration {
    (0..runs)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed()
        })
        .min()
        .unwrap()
}

fn main() {
    let (src, expected) = generate::program(1, 500_000);
    assert_eq!(lex(&src).as_ref(), Ok(&expected));
    assert_eq!(lex_regex(&src).as_ref(), Ok(&expected));
    let mb = src.len() as f64 / 1e6;
    println!("программа: {mb:.1} МБ, {} токенов", expected.len());

    let fsm = best_of(5, || drop(black_box(lex(black_box(&src)))));
    let re = best_of(3, || drop(black_box(lex_regex(black_box(&src)))));
    for (name, t) in [("автомат", fsm), ("regex", re)] {
        println!("{name:<8} {:8.1} мс {:8.1} МБ/с", t.as_secs_f64() * 1e3, mb / t.as_secs_f64());
    }
    println!("regex медленнее в {:.1} раза", re.as_secs_f64() / fsm.as_secs_f64());
}
