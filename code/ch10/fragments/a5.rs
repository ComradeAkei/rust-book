// Задача 10.A, фрагмент a5: внутренняя необобщённая функция.
// ANCHOR: all
use std::hint::black_box;

fn checksum<D: AsRef<[u8]>>(data: D) -> u32 {
    #[inline(never)]
    fn inner(data: &[u8]) -> u32 {
        data.iter().fold(0u32, |s, &x| s.wrapping_add(x.into()))
    }
    inner(data.as_ref())
}

fn main() {
    let a = checksum(black_box([1u8; 4]));
    let b = checksum(black_box(vec![2u8; 8]));
    let c = checksum(black_box("строка"));
    let e = checksum(black_box(&[3u8, 4][..]));
    println!("{a} {b} {c} {e}");
}
// ANCHOR_END: all
