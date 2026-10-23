//! Обработчик паники без форматирования: та же работа, что в `report`.
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

#[entry]
fn main() -> ! {
    hprintln!("опрос каналов");
    let sum = ch07_panic::run();
    debug::exit(if sum > 0 { debug::EXIT_SUCCESS } else { debug::EXIT_FAILURE });
    loop {}
}

// ANCHOR: silent
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
// ANCHOR_END: silent
