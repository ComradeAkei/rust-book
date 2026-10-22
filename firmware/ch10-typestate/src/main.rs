//! Глава 10 на Cortex-M3: UART0 машины lm3s6965evb через `Uart<S>` с
//! состоянием в типе. Писать можно только во включённый UART, менять
//! делитель — только в выключенный, и проверка ничего не стоит.
//!
//! ```text
//! cd firmware && cargo run --release -p ch10-typestate
//! ```
#![no_std]
#![no_main]

use ch06_regs::Pl011;
use ch10::typestate::{BAUD_115200_12MHZ, Disabled, Enabled, Uart};
use core::fmt::Write;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: main
#[entry]
fn main() -> ! {
    // SAFETY: на lm3s6965evb по этому адресу UART0, и больше его никто не
    // трогает.
    let regs = unsafe { Pl011::at(Pl011::UART0_BASE) };
    let mut uart = Uart::new(regs).enable(BAUD_115200_12MHZ);
    writeln!(
        uart,
        "Uart<Enabled>: {} байта, Uart<Disabled>: {} байта, &Pl011: {} байта",
        size_of::<Uart<'_, Enabled>>(),
        size_of::<Uart<'_, Disabled>>(),
        size_of::<&Pl011>(),
    )
    .ok();
    writeln!(
        uart,
        "IBRD = {}, FBRD = {}, CR = {:#x}",
        regs.ibrd.read(),
        regs.fbrd.read(),
        regs.cr.read()
    )
    .ok();

    let off = uart.disable();
    // Через выключенный UART писать нечем: у Uart<Disabled> нет write_byte.
    // Сообщение идёт по semihosting.
    hprintln!("после disable: CR = {:#x}", regs.cr.read());

    let mut uart = off.enable(BAUD_115200_12MHZ);
    writeln!(uart, "снова включён: CR = {:#x}", regs.cr.read()).ok();
    uart.disable();
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
