//! Счётчик загрузок, который переживает программный сброс.
#![no_std]
#![no_main]

use ch08_sections as _;
use core::mem::MaybeUninit;
use cortex_m::peripheral::SCB;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: uninit
/// Код запуска обнуляет .bss и копирует .data из flash, а секцию .uninit
/// (NOLOAD в link.x из cortex-m-rt) не трогает. После включения питания
/// там мусор, поэтому рядом лежит метка.
#[unsafe(link_section = ".uninit.BOOT")]
static mut BOOT: MaybeUninit<[u32; 2]> = MaybeUninit::uninit();

/// Обычная статическая переменная в .bss: после сброса снова 0.
static mut RUNS: u32 = 0;

const MAGIC: u32 = 0x5EED_B007;
// ANCHOR_END: uninit

// ANCHOR: main
#[entry]
fn main() -> ! {
    let boot = &raw mut BOOT as *mut [u32; 2];
    // SAFETY: прерываний нет, к BOOT и RUNS обращается только этот код.
    // Любые 8 байт — допустимый [u32; 2], поэтому чтение мусора не UB;
    // volatile не даёт компилятору выбросить запись перед сбросом.
    let [magic, count] = unsafe { boot.read_volatile() };
    let count = if magic == MAGIC { count + 1 } else { 0 };
    unsafe { boot.write_volatile([MAGIC, count]) };
    let runs = unsafe { (&raw mut RUNS).read_volatile() };
    unsafe { (&raw mut RUNS).write_volatile(runs + 1) };

    hprintln!(
        "загрузка {}: .uninit {:p}, RUNS = {} по адресу {:p}",
        count,
        boot,
        runs,
        &raw const RUNS
    );
    if count < 2 {
        SCB::sys_reset(); // запрос сброса через AIRCR.SYSRESETREQ
    }
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main
