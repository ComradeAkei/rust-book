// Задача 10.A, фрагмент a1: два типа, три вызова.
// ANCHOR: all
use std::hint::black_box;

#[inline(never)]
fn checksum<T: Copy + Into<u32>>(data: &[T]) -> u32 {
    data.iter().fold(0u32, |s, &x| s.wrapping_add(x.into()))
}

fn main() {
    let a = checksum(black_box(&[1u8, 2, 3][..]));
    let b = checksum(black_box(&[4u16, 5][..]));
    let c = checksum(black_box(&[6u8][..]));
    println!("{a} {b} {c}");
}
// ANCHOR_END: all
