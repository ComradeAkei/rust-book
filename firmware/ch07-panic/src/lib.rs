//! Глава 7, раздел 7.8: паника без операционной системы.
//!
//! Обе прошивки выполняют одну и ту же работу из `run`: читают таблицу
//! коэффициентов по индексу, который приходит «снаружи», и на пятом шаге
//! выходят за её границу. Различаются они только обработчиком паники:
//! `report` печатает сообщение и место, `silent` молча завершает QEMU.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch07-panic --bin report                               # Cortex-M3
//! cargo run --release -p ch07-panic --bin report --target thumbv6m-none-eabi   # Cortex-M0
//! cargo run --release -p ch07-panic --bin silent
//! ```
#![no_std]

use cortex_m_semihosting::hprintln;

// ANCHOR: work
/// Коэффициенты усиления каналов АЦП.
pub static GAINS: [u16; 4] = [100, 200, 400, 800];

#[inline(never)]
pub fn gain(channel: usize) -> u16 {
    GAINS[channel] // проверка границ: при channel >= 4 — паника
}

/// Сумма коэффициентов пяти каналов. Строки печатаются без
/// форматирования (`hprintln!` с одним литералом), чтобы `core::fmt`
/// попадал в прошивку только из обработчика паники.
pub fn run() -> u32 {
    let mut sum = 0;
    for ch in 0..=4 {
        // black_box прячет индекс от оптимизатора, как если бы номер
        // канала пришёл по UART.
        sum += gain(core::hint::black_box(ch)) as u32;
        hprintln!("канал прочитан");
    }
    sum
}
// ANCHOR_END: work
