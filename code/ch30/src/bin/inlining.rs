//! Раздел 30.5: вызывающие функции из `inlining` в программе — чтобы
//! сравнить их код без LTO и с LTO.
//!
//! ```text
//! cargo build -p ch30 --release --bin inlining
//! cargo build -p ch30 --profile release-lto --bin inlining
//! ```

use ch30::inlining::{
    call_scale, call_sum, call_sum_generic, call_sum_inline, check_cold, check_hint, check_plain,
};
use std::hint::black_box;

fn main() {
    let v: Vec<u32> = (0..1000).collect();
    let v = black_box(&v[..]);
    println!(
        "{} {} {} {}",
        call_scale(black_box(7)),
        call_sum(v),
        call_sum_inline(v),
        call_sum_generic(v)
    );
    // Нагрузка для PGO: все значения в диапазоне, ветка с отчётом не
    // выполняется ни разу.
    let mut acc = 0u32;
    for i in 0..1_000_000u32 {
        let x = black_box(i % 1000);
        acc = acc.wrapping_add(check_plain(x) ^ check_cold(x) ^ check_hint(x));
    }
    println!("{acc}");
}
