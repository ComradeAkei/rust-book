//! Глава 0: одни и те же функции крейта `ch00` на Cortex-M0, M3 и M4F.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch00-hello --target thumbv6m-none-eabi      # micro:bit, Cortex-M0
//! cargo run --release -p ch00-hello                                  # lm3s6965evb, Cortex-M3
//! cargo run --release -p ch00-hello --target thumbv7em-none-eabihf   # mps2-an386, Cortex-M4F
//! ```
#![no_std]
#![no_main]

use ch00::abi::{ch00_args8, ch00_keep, ch00_mul_wide, ch00_scale};
use core::hint::black_box;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: cpuid
/// Регистр CPUID блока управления системой (SCB) есть на любом Cortex-M:
/// производитель в битах 31..24, номер ядра в битах 15..4.
const CPUID: *const u32 = 0xE000_ED00 as *const u32;

fn core_name(cpuid: u32) -> &'static str {
    match (cpuid >> 4) & 0xFFF {
        0xC20 => "Cortex-M0",
        0xC60 => "Cortex-M0+",
        0xC23 => "Cortex-M3",
        0xC24 => "Cortex-M4",
        0xC27 => "Cortex-M7",
        _ => "неизвестное ядро",
    }
}
// ANCHOR_END: cpuid

extern "C" fn double(x: u32) -> u32 {
    x * 2
}

// ANCHOR: main
#[entry]
fn main() -> ! {
    // SAFETY: CPUID доступен только на чтение и существует на любом Cortex-M.
    let cpuid = unsafe { CPUID.read_volatile() };
    hprintln!("{} (CPUID = {:#010x})", core_name(cpuid), cpuid);
    // black_box не даёт посчитать результаты при компиляции: умножение и
    // плавающая точка должны выполниться на эмулируемом ядре.
    let m = ch00_mul_wide(black_box(0xFFFF_FFFF), black_box(0xFFFF_FFFF));
    hprintln!("mul_wide = {:#018x}", m);
    hprintln!("args8    = {:#010x}", ch00_args8(1, 2, 3, 4, 5, 6, 7, black_box(8)));
    hprintln!("keep     = {}", ch00_keep(black_box(3), double));
    let s = ch00_scale(black_box(1.5), black_box(2.0));
    hprintln!("scale    = {} (биты {:#010x})", s, s.to_bits());
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
