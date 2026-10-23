//! Глава 24, задача A: прошивка, в которой работают два процедурных
//! макроса, — встроенный `#[derive(Debug)]` и `#[entry]` из cortex-m-rt.
//! Предскажите их раскрытие и сверьте:
//!
//! ```text
//! cd firmware
//! RUSTC_BOOTSTRAP=1 cargo rustc -p ch24-expand --profile check -- -Zunpretty=expanded
//! ```
//!
//! Под QEMU прошивка печатает отладочный вывод двух значений и счётчик из
//! `static mut` внутри `main`.
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: types
// Поля читает только Debug, а реализации из derive анализ мёртвого кода
// намеренно не учитывает: без expect были бы предупреждения о полях.
#[derive(Debug)]
#[expect(dead_code, reason = "поля читает только derive(Debug)")]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
#[expect(dead_code, reason = "поле Blink читает только derive(Debug)")]
enum Mode {
    Off,
    Blink(u8),
}
// ANCHOR_END: types

// ANCHOR: main
#[entry]
fn main() -> ! {
    static mut BOOTS: u32 = 0;
    *BOOTS += 1;
    hprintln!("{:?} {:?} {:?}", Point { x: 3, y: -4 }, Mode::Off, Mode::Blink(2));
    hprintln!("BOOTS = {}", BOOTS);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
