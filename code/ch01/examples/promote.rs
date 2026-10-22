//! Раздел 1.1: что в C делает integer promotion и как то же записать в Rust.
//!
//! ```text
//! cargo run -p ch01 --example promote
//! ```
use std::hint::black_box;

// ANCHOR: promote
fn main() {
    let a: u8 = black_box(200);
    let b: u8 = black_box(100);
    // `a + b` имеет тип u8: в dev здесь была бы паника, в release — 44.
    println!("a.wrapping_add(b)      = {}", a.wrapping_add(b));
    println!("a.checked_add(b)       = {:?}", a.checked_add(b));
    println!("u16::from(a) + u16::from(b) = {}", u16::from(a) + u16::from(b));

    // В C `x * x` для uint16_t считается в int и переполняет его: UB.
    let x: u16 = black_box(0xFFFF);
    println!("x.checked_mul(x)       = {:?}", x.checked_mul(x));
    println!("u32::from(x) * u32::from(x) = {}", u32::from(x) * u32::from(x));

    // Побитовое НЕ в C превращает uint8_t 0x0F в int 0xFFFFFFF0.
    let m: u8 = black_box(0x0F);
    println!("!m                     = {:#04x}", !m);
}
// ANCHOR_END: promote
