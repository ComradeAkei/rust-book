//! Глава 26, раздел 26.5: вызовы memcpy и memset, которых нет в
//! исходнике. Их вставил компилятор, а определения пришли из
//! compiler_builtins — слабыми символами.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch26-bare --bin mem
//! llvm-nm -S target/thumbv7m-none-eabi/release/mem | grep -E 'mem|aeabi'
//! ```
#![no_std]
#![no_main]

use ch26::copies::{
    Block, ch26_c_len, ch26_clear, ch26_copy_frame, ch26_fill, ch26_same, ch26_store,
};
use ch26_bare::{entry, sh};
use core::hint::black_box;

entry!(main);

// ANCHOR: main
fn main() -> ! {
    let src: [u8; 40] = core::array::from_fn(|i| black_box(i as u8 * 3));
    let mut frame = [0u8; 64];
    let n = ch26_copy_frame(&mut frame, &src);
    sh::put("copy_from_slice: ");
    sh::dec(n as u32);
    sh::put(" байт, frame[39] = ");
    sh::dec(frame[39].into());
    sh::put("\n");

    let b = Block { words: core::array::from_fn(|i| black_box(i as u32)) };
    let mut slot = Block { words: [0; 64] };
    ch26_store(&mut slot, &b);
    sh::put("присваивание 256 байт: сумма ");
    sh::dec(slot.words.iter().sum());
    ch26_clear(&mut slot);
    sh::put(", после обнуления ");
    sh::dec(slot.words.iter().sum());
    sh::put("\n");

    ch26_fill(&mut frame[..10], 0x5A);
    let mut other = frame;
    let word = |same| if same { "равны" } else { "разные" };
    sh::put("сравнение 32 байт: ");
    sh::put(word(ch26_same(first32(&frame), first32(&other))));
    other[31] ^= 1;
    sh::put(", после правки ");
    sh::put(word(ch26_same(first32(&frame), first32(&other))));
    sh::put("\n");

    // SAFETY: строка C оканчивается нулём.
    let len = unsafe { ch26_c_len(c"станция".as_ptr().cast()) };
    sh::put("strlen(\"станция\") = ");
    sh::dec(len as u32);
    sh::put("\n");
    sh::exit(true)
}
// ANCHOR_END: main

/// Первые 32 байта как массив. `try_into().unwrap()` потянул бы в
/// прошивку `core::fmt` ради `Debug` у ошибки.
fn first32(b: &[u8; 64]) -> &[u8; 32] {
    b.first_chunk().unwrap()
}
