//! Раздел 12.5: калибровка 4096 отсчётов тремя способами, как в C-примере
//! введения: обобщённая функция со своей копией для замыкания, одна копия
//! для `&mut dyn FnMut` и указатель на функцию.
//!
//! Запуск: `cargo run --release -p ch12 --example bench [повторов]`.
//! Время меняется от машины к машине и от запуска к запуску.
use ch12::codegen::{Affine, ch12_map_fn, ch12_scale_dyn, ch12_scale_generic};
use std::hint::black_box;
use std::time::Instant;

const N: usize = 4096;

fn calibrate(x: i16) -> i16 {
    x.wrapping_mul(3).wrapping_add(1)
}

fn measure(reps: u32, mut f: impl FnMut(&mut [i16])) -> (f64, [i16; N]) {
    let mut a: [i16; N] = std::array::from_fn(|i| i as i16);
    let t = Instant::now();
    for _ in 0..reps {
        f(black_box(&mut a));
    }
    let ns = t.elapsed().as_secs_f64() * 1e9 / f64::from(reps) / N as f64;
    (ns, a)
}

fn main() {
    let reps = std::env::args().nth(1).map_or(20_000, |s| s.parse().unwrap());
    let p = Affine { k: 3, b: 1 };
    let (generic, a) = measure(reps, |a| ch12_scale_generic(a, black_box(&p)));
    let (dynamic, b) = measure(reps, |a| ch12_scale_dyn(a, black_box(&p)));
    let (pointer, c) = measure(reps, |a| ch12_map_fn(a, black_box(calibrate)));
    assert!(a == b && b == c, "результаты разошлись");
    println!("обобщённая:     {generic:.2} нс на отсчёт");
    println!("&mut dyn FnMut: {dynamic:.2} нс на отсчёт");
    println!("fn-указатель:   {pointer:.2} нс на отсчёт");
}
