//! Раздел 30.6: цена проверки границ, которая осталась в цикле, проверки
//! переполнения и флага `RefCell`.
//!
//! ```text
//! cargo run -p ch30 --release --bin bounds
//! ```

use ch30::bounds::{hist_array, hist_slice, sum_indexed, sum_iter};
use ch30::checks::{
    hist_cell, hist_cell_call, hist_refcell, hist_refcell_call, hist_refcell_once, sum_strict,
    sum_wide, sum_wrapping,
};
use ch30::hot::noise;
use ch30::measure::bench;
use std::cell::{Cell, RefCell};
use std::hint::black_box;

/// Вес байта для гистограмм с вызовом: функция за указателем, который
/// оптимизатор не видит насквозь.
fn weight(b: u8) -> u32 {
    u32::from(b & 1) + 1
}

fn main() {
    let data = noise(1 << 24, 42);
    let v: Vec<u32> = (0..1 << 20).collect();
    let per = |ns: f64, n: usize| ns / n as f64;
    let mut h = vec![0u32; 256];
    let s = bench(11, 1, || hist_slice(black_box(&data), black_box(&mut h)));
    println!("hist_slice: {:.3} нс на байт", per(s.median, data.len()));
    let mut a = [0u32; 256];
    let s = bench(11, 1, || hist_array(black_box(&data), black_box(&mut a)));
    println!("hist_array: {:.3} нс на байт", per(s.median, data.len()));
    let s = bench(11, 10, || sum_indexed(black_box(&v), v.len()));
    println!("sum_indexed: {:.3} нс на число", per(s.median, v.len()));
    let s = bench(11, 10, || sum_iter(black_box(&v), v.len()));
    println!("sum_iter:    {:.3} нс на число", per(s.median, v.len()));
    // Сумма не должна переполниться: числа до 4095, их 2²⁰ — около 2,1·10⁹.
    let small: Vec<u32> = v.iter().map(|&x| x & 0xfff).collect();
    let s = bench(11, 10, || sum_wrapping(black_box(&small)));
    println!("sum_wrapping: {:.3} нс на число", per(s.median, small.len()));
    let s = bench(11, 10, || sum_strict(black_box(&small)));
    println!("sum_strict:   {:.3} нс на число", per(s.median, small.len()));
    let s = bench(11, 10, || sum_wide(black_box(&small)));
    println!("sum_wide:     {:.3} нс на число", per(s.median, small.len()));
    let rc = RefCell::new([0u32; 256]);
    let s = bench(11, 1, || hist_refcell(black_box(&data), black_box(&rc)));
    println!("hist_refcell:      {:.3} нс на байт", per(s.median, data.len()));
    let s = bench(11, 1, || hist_refcell_once(black_box(&data), black_box(&rc)));
    println!("hist_refcell_once: {:.3} нс на байт", per(s.median, data.len()));
    let c: [Cell<u32>; 256] = std::array::from_fn(|_| Cell::new(0));
    let s = bench(11, 1, || hist_cell(black_box(&data), black_box(&c)));
    println!("hist_cell:         {:.3} нс на байт", per(s.median, data.len()));
    let w: fn(u8) -> u32 = black_box(weight);
    let s = bench(11, 1, || hist_refcell_call(black_box(&data), black_box(&rc), w));
    println!("hist_refcell_call: {:.3} нс на байт", per(s.median, data.len()));
    let s = bench(11, 1, || hist_cell_call(black_box(&data), black_box(&c), w));
    println!("hist_cell_call:    {:.3} нс на байт", per(s.median, data.len()));
}
