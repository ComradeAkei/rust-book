//! Глава 0, задача D: прошивка, в которой почти ничего не происходит. Всё
//! интересное случается до `main`: сценарий gdb из решения проходит путь от
//! вектора сброса до первой строки `main` и показывает каждый шаг.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch00-reset --target thumbv7em-none-eabihf   # mps2-an386, Cortex-M4F
//! ```
#![no_std]
#![no_main]

use core::hint::black_box;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: statics
/// Нулевое начальное значение: переменная попадает в `.bss`, и в образе
/// прошивки для неё нет ни байта.
static mut BOOTS: u32 = 0;

/// Ненулевое начальное значение: переменная попадает в `.data`. Значение
/// хранится во флеш-памяти и копируется в ОЗУ до вызова `main`.
static mut SIGNATURE: u32 = 0x5445_4C45; // «TELE» в ASCII
// ANCHOR_END: statics

// ANCHOR: main
#[entry]
fn main() -> ! {
    // SAFETY: прерываний нет, к переменным обращается только этот код.
    let (boots, sig) = unsafe {
        BOOTS += 1;
        ((&raw const BOOTS).read_volatile(), (&raw const SIGNATURE).read_volatile())
    };
    // На Cortex-M4F это инструкции FPU. Если сопроцессор не включён, первая
    // же из них вызовет UsageFault с флагом NOCP.
    let kilo = (black_box(sig as f32) / black_box(1000.0f32)) as u32;
    hprintln!("BOOTS = {}, SIGNATURE = {:#010x}, SIGNATURE / 1000 = {}", boots, sig, kilo);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
