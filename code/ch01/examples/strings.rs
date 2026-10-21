//! Раздел 1.6: строка — это байты UTF-8, и резать её можно только по
//! границе символа.
//!
//! ```text
//! cargo run -p ch01 --example strings
//! ```
use std::hint::black_box;

// ANCHOR: strings
fn main() {
    let s = "привет";
    println!("len = {}, chars = {}", s.len(), s.chars().count());
    println!("байты: {:02x?}", s.as_bytes());
    let end = black_box(1);
    println!("{}", &s[..end]);
}
// ANCHOR_END: strings
