//! Глава 30, раздел 30.8: чем мерить такты на Cortex-M и что из этого
//! есть в QEMU.
//!
//! На настоящем Cortex-M3/M4 такты считает DWT_CYCCNT. В QEMU блока DWT
//! нет: регистры читаются нулями, и счётчик стоит. SysTick в QEMU есть, а
//! с `-icount` виртуальное время идёт по выполненным инструкциям, поэтому
//! SysTick отсчитывает инструкции — детерминированно, но не такты.
//!
//! ```text
//! cd firmware
//! cargo build --release -p ch30-cycles
//! qemu-system-arm -cpu cortex-m3 -machine lm3s6965evb -nographic \
//!     -semihosting-config enable=on,target=native -icount shift=6 \
//!     -kernel target/thumbv7m-none-eabi/release/ch30-cycles
//! cargo build --profile release-s -p ch30-cycles     # opt-level = "s"
//! ```
#![no_std]
#![no_main]

use core::hint::black_box;
use cortex_m::peripheral::{DWT, SYST, syst::SystClkSource};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use telemetry_frame::{crc16_update, crc16_update_table};

// ANCHOR: spin
/// Ровно `2 * n` инструкций: вычитание и переход назад.
fn spin(n: u32) {
    // SAFETY: цикл трогает только свой регистр и флаги.
    unsafe {
        core::arch::asm!("1: subs {0}, #1", "bne 1b", inout(reg) n => _, options(nomem, nostack));
    }
}
// ANCHOR_END: spin

// ANCHOR: ticks
/// Сколько тиков SysTick прошло за `f`. Счётчик 24-битный и считает
/// вниз; замер короче периода, поэтому хватает разности по модулю 2^24.
fn ticks<R>(f: impl FnOnce() -> R) -> u32 {
    let t0 = SYST::get_current();
    black_box(f());
    let t1 = SYST::get_current();
    t0.wrapping_sub(t1) & 0x00FF_FFFF
}
// ANCHOR_END: ticks

#[inline(never)]
fn crc_bitwise(d: &[u8]) -> u16 {
    crc16_update(0xFFFF, d)
}

#[inline(never)]
fn crc_table(d: &[u8]) -> u16 {
    crc16_update_table(0xFFFF, d)
}

#[entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();

    // ANCHOR: dwt
    cp.DCB.enable_trace();
    DWT::unlock();
    cp.DWT.enable_cycle_counter();
    let c0 = DWT::cycle_count();
    spin(1000);
    let c1 = DWT::cycle_count();
    // SAFETY: DWT_CTRL — регистр только для чтения в части битов, чтение
    // не меняет состояния.
    let ctrl = unsafe { (0xE000_1000 as *const u32).read_volatile() };
    hprintln!(
        "DWT: has_cycle_counter() = {}, DWT_CTRL = {:#010x}, CYCCNT {} -> {}",
        DWT::has_cycle_counter(),
        ctrl,
        c0,
        c1
    );
    // ANCHOR_END: dwt

    // ANCHOR: systick
    cp.SYST.set_clock_source(SystClkSource::Core);
    cp.SYST.set_reload(0x00FF_FFFF);
    cp.SYST.clear_current();
    cp.SYST.enable_counter();
    // После clear_current счётчик равен нулю до первой перезагрузки.
    while SYST::get_current() == 0 {}
    hprintln!(
        "SysTick: пустой замер {} тиков, spin(1000) {} тиков",
        ticks(|| ()),
        ticks(|| spin(1000))
    );
    // ANCHOR_END: systick

    // ANCHOR: crc
    let mut data = [0u8; 256];
    for (i, b) in data.iter_mut().enumerate() {
        *b = (i * 7 + 3) as u8;
    }
    let d = black_box(&data[..]);
    let (bitwise, table) = (ticks(|| crc_bitwise(d)), ticks(|| crc_table(d)));
    assert_eq!(crc_bitwise(d), crc_table(d));
    hprintln!(
        "opt-level = {}: CRC по 256 байтам — побитово {} тиков, по таблице {} тиков",
        env!("CH30_OPT_LEVEL"),
        bitwise,
        table
    );
    // ANCHOR_END: crc
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
