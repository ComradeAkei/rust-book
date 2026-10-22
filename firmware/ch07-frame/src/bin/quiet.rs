//! Реакция на ошибку без её сообщения: `core::fmt` в прошивку не попадает.
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: quiet
#[entry]
fn main() -> ! {
    if ch07_frame::second_frame().is_err() {
        hprintln!("кадр отвергнут");
    }
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: quiet
