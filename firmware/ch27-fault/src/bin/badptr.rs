//! Глава 27, раздел 27.7: вызов через испорченный указатель на функцию.
//! В таблице обработчиков вместо адреса функции — чётное число, и
//! процессор, перейдя по нему, сбрасывает бит Thumb. Следующая же
//! инструкция — UsageFault INVSTATE, а без включённого UsageFault —
//! HardFault. Обработчик печатает сохранённый кадр.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-fault --bin badptr
//! ```
#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, entry, exception};
use cortex_m_semihosting::{debug, hprintln};

fn ping() -> u32 {
    7
}

// ANCHOR: table
/// Таблица обработчиков команд. Второй элемент «испорчен»: так выглядит
/// указатель, переписанный чужой записью в память.
static HANDLERS: [usize; 2] = [0, 0x0000_0100];

#[inline(never)]
fn dispatch(cmd: usize) -> u32 {
    let addr = if cmd == 0 { ping as *const () as usize } else { HANDLERS[cmd] };
    // SAFETY: нарушена намеренно — адрес из HANDLERS[1] не функция.
    let f: fn() -> u32 = unsafe { core::mem::transmute(addr) };
    f()
}
// ANCHOR_END: table

#[entry]
fn main() -> ! {
    hprintln!("команда 0: {}", dispatch(core::hint::black_box(0)));
    hprintln!("команда 1: {}", dispatch(core::hint::black_box(1)));
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}

// ANCHOR: hardfault
#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    // SAFETY: регистры состояния сбоя только читаются.
    let (cfsr, hfsr) = unsafe { ((*SCB::PTR).cfsr.read(), (*SCB::PTR).hfsr.read()) };
    hprintln!("HardFault: pc {:#010x}, lr {:#010x}, xpsr {:#010x}", ef.pc(), ef.lr(), ef.xpsr());
    hprintln!("CFSR {:#010x}, HFSR {:#010x}", cfsr, hfsr);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: hardfault

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
