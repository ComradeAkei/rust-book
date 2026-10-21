//! Глава 0, задача D: что будет, если не включить FPU. Прошивка сама
//! закрывает сопроцессоры CP10 и CP11, которые открыл Reset, и выполняет
//! одну инструкцию с плавающей точкой.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch00-reset --bin nocp --target thumbv7em-none-eabihf
//! ```
#![no_std]
#![no_main]

use core::hint::black_box;
use cortex_m_rt::{ExceptionFrame, entry, exception};
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: nocp
/// Coprocessor Access Control Register: биты 23..20 открывают CP10 и CP11,
/// то есть FPU.
const CPACR: *mut u32 = 0xE000_ED88 as *mut u32;

#[entry]
fn main() -> ! {
    // SAFETY: CPACR есть на Cortex-M4F; после записи нужны барьеры, чтобы
    // следующая инструкция уже видела новое значение.
    unsafe {
        hprintln!("CPACR после Reset: {:#010x}", CPACR.read_volatile());
        CPACR.write_volatile(0);
        core::arch::asm!("dsb", "isb");
    }
    hprintln!("FPU закрыт, складываем два f32");
    let x = black_box(1.5f32) + black_box(2.25f32);
    hprintln!("не дойдём сюда: {}", x as u32);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    // SAFETY: адреса CFSR и HFSR из ARMv7-M Architecture Reference Manual.
    let (cfsr, hfsr) = unsafe {
        ((0xE000_ED28 as *const u32).read_volatile(), (0xE000_ED2C as *const u32).read_volatile())
    };
    hprintln!("HardFault: PC = {:#010x}, CFSR = {:#010x}, HFSR = {:#010x}", ef.pc(), cfsr, hfsr);
    if cfsr & (1 << 19) != 0 {
        hprintln!("UFSR.NOCP: инструкция сопроцессора, а сопроцессор закрыт");
    }
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
// ANCHOR_END: nocp

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
