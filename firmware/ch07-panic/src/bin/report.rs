//! Обработчик паники, который печатает сообщение и место по semihosting.
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

// ANCHOR: report
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Прерывания больше не нужны: состояние программы уже неизвестно.
    cortex_m::interrupt::disable();
    if let Some(loc) = info.location() {
        hprintln!("паника в {}:{}:{}", loc.file(), loc.line(), loc.column());
    }
    hprintln!("{}", info.message());
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
// ANCHOR_END: report
