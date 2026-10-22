//! Глава 5, раздел 5.3: где лежат данные со временем жизни `'static`.
//!
//! Прошивка печатает адреса строкового литерала, неизменяемых и изменяемых
//! статических переменных, буфера из `singleton!` и локального массива и
//! по карте памяти (`memory.x`) определяет, flash это или ОЗУ.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch05-addr
//! ```
#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: statics
/// Неизменяемые статические данные: ссылка и сами байты строки.
static BANNER: &str = "станция телеметрии";
static LIMITS: [u16; 4] = [100, 200, 400, 800];

/// Константа — значение без адреса. `&GAINS` превращает его в безымянную
/// статическую переменную (продвижение константы).
const GAINS: [u16; 4] = [1, 2, 4, 8];

/// Изменяемые статические переменные. Нулевая попадает в `.bss`,
/// ненулевая — в `.data`: её начальное значение лежит во flash и
/// копируется в ОЗУ до `main`.
static BOOTS: AtomicU32 = AtomicU32::new(0);
static SERIAL: AtomicU32 = AtomicU32::new(0x5EED);
// ANCHOR_END: statics

/// Регион по карте памяти из `memory.x`.
fn region(addr: usize) -> &'static str {
    match addr {
        0x0000_0000..0x0004_0000 => "flash",
        0x2000_0000..0x2000_4000 => "ОЗУ",
        _ => "?",
    }
}

fn show<T: ?Sized>(what: &str, p: *const T) {
    let addr = p.cast::<u8>() as usize;
    hprintln!("{:<24} {:#010x}  {}", what, addr, region(addr));
}

// ANCHOR: main
#[entry]
fn main() -> ! {
    let local = [0u8; 16];
    // `&'static mut` на буфер в ОЗУ: singleton! выдаёт его один раз.
    let buf: &'static mut [u8; 64] = cortex_m::singleton!(: [u8; 64] = [0; 64]).unwrap();
    let gains: &'static [u16; 4] = &GAINS;
    // fetch_add нет на Cortex-M0 (ARMv6-M без LDREX/STREX), а прерываний
    // здесь нет: хватает загрузки и записи.
    BOOTS.store(BOOTS.load(Relaxed) + 1, Relaxed);

    show("байты BANNER", BANNER.as_ptr());
    show("static BANNER (&str)", &BANNER);
    show("static LIMITS", &LIMITS);
    show("&GAINS (const)", gains);
    show("static BOOTS (.bss)", &BOOTS);
    show("static SERIAL (.data)", &SERIAL);
    show("singleton! буфер", buf);
    show("локальный массив", &local);
    hprintln!("BOOTS = {}, SERIAL = {:#x}", BOOTS.load(Relaxed), SERIAL.load(Relaxed));

    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
