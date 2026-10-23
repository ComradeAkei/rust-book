// Задача 10.A, фрагмент a4: параметр-константа.
// ANCHOR: all
use std::hint::black_box;

#[inline(never)]
fn checksum<const N: usize>(data: &[u8; N]) -> u32 {
    data.iter().fold(0u32, |s, &x| s.wrapping_add(x.into()))
}

fn main() {
    let a = checksum(black_box(&[1u8; 4]));
    let b = checksum(black_box(&[2u8; 8]));
    let c = checksum(black_box(&[3u8; 4]));
    let e = checksum(black_box(&[4u8; 64]));
    println!("{a} {b} {c} {e}");
}
// ANCHOR_END: all
