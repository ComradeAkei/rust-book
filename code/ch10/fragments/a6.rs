// Задача 10.A, фрагмент a6: компоновщик со слиянием одинакового кода.
// rustc: -C link-arg=-Wl,--icf=all
// ANCHOR: all
use std::hint::black_box;

#[inline(never)]
fn checksum<T: Copy + Into<u32>>(data: &[T]) -> u32 {
    data.iter().fold(0u32, |s, &x| s.wrapping_add(x.into()))
}

fn main() {
    let a = checksum(black_box(&[1u32, 2][..]));
    let b = checksum(black_box(&['a', 'b'][..]));
    let c = checksum(black_box(&[1u16, 2][..]));
    println!("{a} {b} {c}");
}
// ANCHOR_END: all
