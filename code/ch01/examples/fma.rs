//! Раздел 1.4: `mul_add` без аппаратной FMA и с ней.
//!
//! ```text
//! cargo asm -p ch01 --example fma fma::fused                         # вызов fmaf
//! cargo asm -p ch01 --example fma --target-cpu=x86-64-v3 fma::fused  # vfmadd
//! ```
use std::hint::black_box;

// ANCHOR: fused
#[inline(never)]
fn fused(a: f32, b: f32, c: f32) -> f32 {
    a.mul_add(b, c)
}
// ANCHOR_END: fused

fn main() {
    println!("{}", fused(black_box(1.5), black_box(2.0), black_box(0.25)));
}
