//! Раздел 0.3: почему функция исчезает из ассемблера и как её удержать.
//!
//! ```text
//! cargo asm -p ch00 --example fold main
//! cargo asm -p ch00 --example fold square_kept
//! ```
use std::hint::black_box;

// ANCHOR: fold
fn square(x: u32) -> u32 {
    x * x
}

#[inline(never)]
fn square_kept(x: u32) -> u32 {
    x * x
}

fn main() {
    let a = square(12); // встроится и посчитается при компиляции
    let b = square_kept(black_box(12)); // останется вызовом
    println!("{a} {b}");
}
// ANCHOR_END: fold
