//! Сколько стека стоит сборка блока в 1 КиБ по значению и через `&mut`.
#![no_std]
#![no_main]

use ch03::moves::{Block, build_by_value, build_in_place};
use ch03_stack::{stack_bottom, stack_top, stack_used};
use core::hint::black_box;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: main
#[entry]
fn main() -> ! {
    hprintln!(
        "стек: {:#010x}..{:#010x}, {} байт",
        stack_bottom(),
        stack_top(),
        stack_top() - stack_bottom()
    );

    let (crc, used) = stack_used(|| build_by_value(black_box(7), 0xA5).crc);
    hprintln!("по значению: crc = {:#x}, стек {} байт", crc, used);

    let (crc, used) = stack_used(|| {
        let mut b = Block::empty();
        build_in_place(black_box(&mut b), black_box(7), 0xA5);
        b.crc
    });
    hprintln!("через &mut:  crc = {:#x}, стек {} байт", crc, used);

    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
