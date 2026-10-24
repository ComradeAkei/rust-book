//! Глава 26, разделы 26.3–26.4: прошивка без cortex-m-rt.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch26-bare
//! CH26_NO_GUARD=1 cargo run --release -p ch26-bare --features orphan-data
//! ```
#![no_std]
#![no_main]

use ch26_bare::{VECTORS, entry, sh};
use core::arch::asm;

entry!(main);

// ANCHOR: orphan
#[cfg(feature = "orphan-data")]
static mut COUNTER: u32 = 5;
// ANCHOR_END: orphan

// ANCHOR: main
fn main() -> ! {
    sh::put("ch26-bare: без cortex-m-rt, без .data и .bss\n");
    let sp: u32;
    // SAFETY: чтение SP ничего не меняет.
    unsafe { asm!("mov {}, sp", out(reg) sp, options(nomem, nostack, preserves_flags)) };
    sh::put("SP в main: ");
    sh::hex(sp);
    sh::put("\n");
    // Таблица векторов — по адресу 0: её читает процессор при сбросе. Там
    // лежит объект Rust, VECTORS, и читать его надо через указатель на
    // него: у указателя из числа 0 нет provenance (раздел 16.4).
    let table = (&raw const VECTORS).cast::<u32>();
    // SAFETY: первые два слова VECTORS — указатель и адрес функции.
    let (w0, w1) = unsafe { (table.read_volatile(), table.add(1).read_volatile()) };
    sh::put("слово 0: ");
    sh::hex(w0);
    sh::put(", слово 1: ");
    sh::hex(w1);
    sh::put("\n");
    sh::put("VECTORS по адресу ");
    sh::hex(&raw const VECTORS as u32);
    sh::put(", Reset по адресу ");
    sh::hex(ch26_bare::Reset as *const () as u32);
    sh::put("\n");
    #[cfg(feature = "orphan-data")]
    // SAFETY: прерываний нет, к COUNTER обращается только этот код.
    unsafe {
        COUNTER += 1;
        sh::put("COUNTER = ");
        sh::dec((&raw const COUNTER).read_volatile());
        sh::put(" (ожидалось 6)\n");
    }
    sh::exit(true)
}
// ANCHOR_END: main
