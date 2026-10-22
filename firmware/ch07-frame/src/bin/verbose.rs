//! Та же работа, что в `quiet`, но ошибка печатается целиком цепочкой
//! `Display` -> `source`.
#![no_std]
#![no_main]

use core::error::Error;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: verbose
#[entry]
fn main() -> ! {
    if let Err(e) = ch07_frame::second_frame() {
        hprintln!("ошибка: {}", e);
        let mut cause = e.source();
        while let Some(c) = cause {
            hprintln!("  причина: {}", c);
            cause = c.source();
        }
    }
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: verbose
