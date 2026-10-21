//! Раздел 1.4: поля чисел IEEE 754 и шаг между соседними значениями.
//!
//! ```text
//! cargo run -p ch01 --example floats
//! ```
use ch01::floats::{fields, ulp};

// ANCHOR: floats
fn main() {
    for x in [
        1.0f32,
        -2.5,
        0.1,
        16_777_216.0,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::INFINITY,
        f32::NAN,
    ] {
        let f = fields(x);
        println!(
            "{:>14e}  {:#010x}  знак {}  порядок {:3}  мантисса {:#08x}  ulp {:e}",
            x,
            x.to_bits(),
            f.negative as u8,
            f.biased_exp,
            f.fraction,
            ulp(x)
        );
    }
}
// ANCHOR_END: floats
