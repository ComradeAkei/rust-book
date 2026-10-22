//! Цена непредсказуемого перехода. Одна и та же функция с таблицей
//! переходов выполняет одинаковое число операций, меняется только порядок
//! кодов операций.
//!
//! ```console
//! $ cargo run -p ch02 --release --example dispatch
//! ```

// ANCHOR: dispatch
use ch02::lowering::ch02_exec;
use std::hint::black_box;
use std::time::Instant;

const OPS: usize = 1 << 16;
const ROUNDS: usize = 200;

/// Прогоняет последовательность кодов через `ch02_exec`. Результат каждой
/// операции — аргумент следующей, так что вычисления не перекрываются.
fn nanos_per_op(ops: &[u8]) -> f64 {
    let start = Instant::now();
    let mut acc = 1u32;
    for _ in 0..ROUNDS {
        for &op in ops {
            acc = ch02_exec(op, acc, 3);
        }
    }
    black_box(acc);
    start.elapsed().as_nanos() as f64 / (ROUNDS * ops.len()) as f64
}

fn main() {
    let same = vec![0u8; OPS];
    let cycle: Vec<u8> = (0..OPS).map(|i| (i % 8) as u8).collect();
    let mut x = 0x2545_F491_u32;
    let random: Vec<u8> = (0..OPS)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 29) as u8
        })
        .collect();

    for (name, ops) in
        [("всегда 0", &same), ("0, 1, …, 7 по кругу", &cycle), ("случайные 0–7", &random)]
    {
        println!("{name:<22} {:5.2} нс на операцию", nanos_per_op(black_box(ops)));
    }
}
// ANCHOR_END: dispatch
