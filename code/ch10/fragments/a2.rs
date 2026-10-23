// Задача 10.A, фрагмент a2: то же без #[inline(never)].
// ANCHOR: all
use std::hint::black_box;

fn checksum<T: Copy + Into<u32>>(data: &[T]) -> u32 {
    data.iter().fold(0u32, |s, &x| s.wrapping_add(x.into()))
}

fn main() {
    let a = checksum(black_box(&[1u8, 2, 3][..]));
    let b = checksum(black_box(&[4u16, 5][..]));
    println!("{a} {b}");
}
// ANCHOR_END: all
