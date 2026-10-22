//! Глава 9 на Cortex-M3 и Cortex-M4F: SysTick через трейт `Register`.
//! Адреса регистров — ассоциированные константы, в коде прошивки они
//! становятся непосредственными операндами, а таймер в QEMU отвечает так
//! же, как настоящий.
//!
//! ```text
//! cd firmware && cargo run --release -p ch09-systick
//! ```
#![no_std]
#![no_main]

use ch09::assoc::{
    Reg, Register, SYST_CSR_COUNTFLAG, SystCsr, SystCvr, SystRvr, ch09_systick_start,
};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: main
const RELOAD: u32 = 99_999;
const PERIODS: u32 = 3;

#[entry]
fn main() -> ! {
    for (name, addr) in [
        (SystCsr::NAME, SystCsr::ADDR),
        (SystRvr::NAME, SystRvr::ADDR),
        (SystCvr::NAME, SystCvr::ADDR),
    ] {
        hprintln!("{} по адресу {:#010x}", name, addr);
    }
    // SAFETY: SysTick есть в каждом Cortex-M, и, кроме этой функции, его
    // никто не трогает.
    unsafe { ch09_systick_start(RELOAD) };
    // SAFETY: чтение регистров SysTick побочных эффектов, кроме сброса
    // COUNTFLAG, не имеет.
    let rvr = unsafe { Reg::<SystRvr>::read() };
    hprintln!("SYST_RVR = {}", rvr);
    for _ in 0..PERIODS {
        // SAFETY: см. выше. Чтение CSR сбрасывает COUNTFLAG, поэтому каждый
        // проход цикла ждёт следующего обнуления счётчика.
        while unsafe { Reg::<SystCsr>::read() } & SYST_CSR_COUNTFLAG == 0 {}
    }
    hprintln!("SysTick: {} периода по {} тактов отсчитаны", PERIODS, RELOAD + 1);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
