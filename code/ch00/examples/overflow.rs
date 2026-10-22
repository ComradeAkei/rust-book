//! Разделы 0.2 и 0.3: одна и та же строка ведёт себя по-разному в профилях
//! dev и release.
//!
//! ```text
//! cargo run -p ch00 --example overflow
//! cargo run -p ch00 --example overflow --release
//! ```

// ANCHOR: overflow
fn main() {
    let level: u8 = std::hint::black_box(255);
    let next = level + 1;
    println!("255 + 1 = {next}");
}
// ANCHOR_END: overflow
