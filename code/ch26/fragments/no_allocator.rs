// Раздел 26.1: alloc без распределителя памяти. Программа для Cortex-M3,
// собирается командой из теста code/ch26/tests/nostd.rs.
// ANCHOR: no_allocator
#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

#[unsafe(no_mangle)]
pub extern "C" fn Reset() -> ! {
    let mut v = Vec::new();
    v.push(core::hint::black_box(1u8));
    core::hint::black_box(&v);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
// ANCHOR_END: no_allocator
