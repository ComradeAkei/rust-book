//! Задача 0.A: какие из пяти функций доживут до release-бинарника?
//!
//! Не запускайте `nm`, пока не запишете ответы в
//! `exercises/ch00/src/a_vanish.rs`. Проверка:
//!
//! ```text
//! cargo build -p ch00 --release --example vanish
//! nm target/release/examples/vanish | grep vanish
//! cargo asm -p ch00 --example vanish
//! ```
use std::hint::black_box;

// ANCHOR: five
/// 1. Один шаг CRC-32, встраивание запрещено.
#[inline(never)]
fn crc_step(seed: u32) -> u32 {
    let mut c = seed;
    for _ in 0..8 {
        c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
    }
    c
}

/// 2. Контрольная сумма буфера, встраивание запрещено.
#[inline(never)]
fn checksum(data: &[u8]) -> u32 {
    data.iter().fold(0u32, |acc, &b| acc.rotate_left(3) ^ b as u32)
}

/// 3. Число единичных битов, встраивание запрещено.
#[inline(never)]
fn popcount(x: u32) -> u32 {
    x.count_ones()
}

/// 4. Функция с фиксированным именем, которую никто не вызывает.
#[unsafe(no_mangle)]
pub fn ch00a_exported(x: u32) -> u32 {
    x ^ 0x5555
}

/// 5. Крошечная функция без атрибутов.
fn triple(x: u32) -> u32 {
    x * 3
}

fn main() {
    let a = crc_step(1);
    let b = checksum(black_box(b"telemetry"));
    let _ = popcount(black_box(0xF0F0));
    let f: fn(u32) -> u32 = black_box(triple);
    let c = f(black_box(14));
    println!("{a:#010x} {b:#010x} {c}");
}
// ANCHOR_END: five
