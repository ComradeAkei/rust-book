//! Раздел 30.3: цель для perf record, flamegraph и cachegrind.
//!
//! ```text
//! cargo build -p ch30 --release --bin profile
//! perf record -e cpu-clock --call-graph dwarf target/release/profile
//! perf report --stdio --no-children
//! ```

use ch30::hot::{crc_bitwise, histogram, noise, sorted};

// ANCHOR: main
fn main() {
    let mb: usize = std::env::args().nth(1).map_or(64, |s| s.parse().unwrap());
    let data = noise(mb << 20, 0x9E37_79B9_7F4A_7C15);
    let crc = crc_bitwise(&data);
    let h = histogram(&data);
    let s = sorted(&data);
    println!("{mb} МиБ: crc {crc:04x}, нулей {}, медиана {}", h[0], s[s.len() / 2]);
}
// ANCHOR_END: main
