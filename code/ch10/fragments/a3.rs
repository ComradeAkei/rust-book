// Задача 10.A, фрагмент a3: замыкания, функция и указатель на функцию.
// ANCHOR: all
use std::hint::black_box;

#[inline(never)]
fn checksum<F: Fn(u8) -> u32>(data: &[u8], f: F) -> u32 {
    data.iter().fold(0u32, |s, &x| s.wrapping_add(f(x)))
}

fn widen(x: u8) -> u32 {
    x.into()
}

fn main() {
    let d = black_box(&[1u8, 2, 3][..]);
    let a = checksum(d, |x| x as u32);
    let b = checksum(d, |x| x as u32);
    let c = checksum(d, widen);
    let e = checksum(d, widen);
    let g = checksum(d, widen as fn(u8) -> u32);
    println!("{a} {b} {c} {e} {g}");
}
// ANCHOR_END: all
