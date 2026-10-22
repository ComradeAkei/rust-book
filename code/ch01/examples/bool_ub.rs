//! Раздел 1.5: байт 2 под видом `bool`. Программа содержит неопределённое
//! поведение намеренно, чтобы показать его последствия и то, как его ловит
//! Miri.
//!
//! ```text
//! cargo run -p ch01 --example bool_ub --release
//! cargo +nightly miri run -p ch01 --example bool_ub
//! ```
use std::hint::black_box;

// ANCHOR: bool_ub
#[expect(clippy::transmute_int_to_bool, reason = "ошибка показана намеренно")]
fn main() {
    let byte = black_box(2u8);
    // Ошибка: у bool допустимы только 0 и 1, а transmute ничего не проверяет.
    let b: bool = unsafe { std::mem::transmute::<u8, bool>(byte) };
    println!("b = {b}, !b = {}", !b);
    println!("b as u8 = {}, !b as u8 = {}", b as u8, !b as u8);
}
// ANCHOR_END: bool_ub
