//! Переполнение стека на Cortex-M: стек молча затирает статические
//! переменные, и никакого исключения нет.
#![no_std]
#![no_main]

use ch03_stack::{dig, stack_bottom, stack_top};
use core::fmt::Write;
use core::ptr::addr_of;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hio};

// ANCHOR: say
/// Печать без `hprintln!`. Тот хранит открытый поток вывода в статической
/// переменной `HSTDOUT`, а она лежит в `.bss` прямо под стеком, и
/// переполнение её портит. Здесь поток открывается заново при каждом
/// вызове.
macro_rules! say {
    ($($arg:tt)*) => {
        if let Ok(mut out) = hio::hstdout() {
            let _ = writeln!(out, $($arg)*);
        }
    };
}
// ANCHOR_END: say

// ANCHOR: statics
/// Настройки с ненулевыми начальными значениями: секция `.data`.
static mut SETTINGS: [u32; 64] = [0x5E77_1495; 64];
/// Счётчики, обнулённые при старте: секция `.bss`, сразу под стеком.
static mut COUNTERS: [u32; 64] = [0; 64];
// ANCHOR_END: statics

/// Сколько слов массива отличается от исходного значения.
fn damaged(words: &[u32; 64], expect: u32) -> usize {
    words.iter().filter(|&&w| w != expect).count()
}

fn show(when: &str) {
    // SAFETY: прерываний нет, к статическим переменным обращается только
    // этот поток; читаем копию через сырой указатель, без ссылок на static mut.
    let (s, c) = unsafe { (addr_of!(SETTINGS).read_volatile(), addr_of!(COUNTERS).read_volatile()) };
    say!("{}: испорчено SETTINGS {}/64, COUNTERS {}/64, COUNTERS[63] = {:#010x}",
              when, damaged(&s, 0x5E77_1495), damaged(&c, 0), c[63]);
}

// ANCHOR: main
#[entry]
fn main() -> ! {
    say!("SETTINGS {:#010x}, COUNTERS {:#010x}, стек {:#010x}..{:#010x}",
              addr_of!(SETTINGS) as usize, addr_of!(COUNTERS) as usize,
              stack_bottom(), stack_top());
    show("до");
    // Останавливаемся на 64 байта ниже границы стека, то есть внутри .bss.
    let levels = dig(0, stack_bottom() - 64);
    say!("рекурсия: {} уровней по 256 байт", levels);
    show("после");
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
