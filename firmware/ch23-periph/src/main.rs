//! Глава 23, задача C, на Cortex-M3 (lm3s6965evb): UART0 описан макросом
//! `peripheral!`, биты его регистров — макросом `flags!` (задача B).
//! Строка уходит в UART0 — в QEMU это стандартный вывод, — а состояние
//! регистров печатается через semihosting.
//!
//! ```text
//! cd firmware && cargo run --release -p ch23-periph
//! ```
//!
//! Собирается под все три цели, но UART0 по адресу 0x4000_C000 есть только
//! у lm3s6965evb — запускается прошивка на M3.
#![no_std]
#![no_main]

use ch23_solutions::{flags, peripheral};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: device
peripheral! {
    /// UART0 машины lm3s6965evb — PL011 (раздел 6.9).
    UART0 @ 0x4000_C000 {
        DR: u32 @ 0x000,
        FR: u32 @ 0x018 ro,
        IBRD: u32 @ 0x024,
        FBRD: u32 @ 0x028,
        LCRH: u32 @ 0x02C,
        CR: u32 @ 0x030,
        ICR: u32 @ 0x044 wo,
    }
}

flags! {
    /// UARTFR: флаги состояния.
    struct Fr: u32 {
        const BUSY = 1 << 3;
        const RXFE = 1 << 4;
        const TXFF = 1 << 5;
        const RXFF = 1 << 6;
        const TXFE = 1 << 7;
    }
}

flags! {
    /// UARTCR: управление.
    struct Cr: u32 {
        const UARTEN = 1 << 0;
        const LBE = 1 << 7;
        const TXE = 1 << 8;
        const RXE = 1 << 9;
    }
}
// ANCHOR_END: device

// ANCHOR: main
#[entry]
fn main() -> ! {
    // SAFETY: прошивка — единственный владелец UART0 на этой машине.
    let uart = unsafe { UART0::steal() };
    hprintln!("CR после сброса: {:?}", Cr::from_bits_retain(uart.CR().read()));
    uart.CR().write((Cr::UARTEN | Cr::TXE | Cr::RXE).bits());
    for &b in b"uart0: hello from peripheral!\n" {
        while Fr::from_bits_retain(uart.FR().read()).contains(Fr::TXFF) {}
        uart.DR().write(u32::from(b));
    }
    uart.ICR().write(0x7FF); // сбросить все прерывания; регистр только для записи
    hprintln!("CR: {:?}", Cr::from_bits_retain(uart.CR().read()));
    // Биты приёмника зависят от стандартного ввода QEMU: он идёт в UART0, и
    // если там что-то есть, вместо RXFE будет RXFF. Передатчик от ввода не
    // зависит — его биты печатаются отдельно.
    let fr = Fr::from_bits_retain(uart.FR().read());
    hprintln!("FR: {:?}", fr);
    hprintln!("FR, передатчик: {:?}", fr & (Fr::BUSY | Fr::TXFF | Fr::TXFE));
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
