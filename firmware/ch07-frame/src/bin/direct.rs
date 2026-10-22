//! Те же сообщения, что в `verbose`, но без `dyn Error`: причина известна
//! статически и печатается напрямую.
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: direct
#[entry]
fn main() -> ! {
    if let Err(e) = ch07_frame::second_frame() {
        hprintln!("ошибка: {}", e);
        hprintln!("  причина: {}", e.error);
    }
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: direct
