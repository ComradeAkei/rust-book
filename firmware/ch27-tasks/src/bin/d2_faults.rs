//! Задача 27.D2: три сбоя UsageFault и продолжение работы после каждого.
//! UsageFault включён (SHCSR.USGFAULTENA), деление на ноль — ловушка
//! (CCR.DIV_0_TRP). Обработчик разбирает CFSR кодом задачи и правит
//! сохранённый кадр: пропускает инструкцию или возвращает бит Thumb.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-tasks --bin d2_faults
//! ```
#![no_std]
#![no_main]

use ch27::exc::Frame;
use ch27_solutions::d2_faults::{faults, insn_len};
use core::arch::{asm, naked_asm};
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use cortex_m::peripheral::SCB;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::{debug, hprintln};

/// CFSR последнего сбоя: обработчик сохраняет, main печатает.
static LAST: AtomicU32 = AtomicU32::new(0);

// ANCHOR: handler
/// Вход в UsageFault: как SVCall в разделе 27.1 — адрес кадра в r0.
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn UsageFault() {
    naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "b {rust}",
        rust = sym usage_rust,
    );
}

extern "C" fn usage_rust(frame: &mut Frame) {
    // SAFETY: CFSR читается и сбрасывается записью единиц в те же биты.
    let cfsr = unsafe {
        let scb = &*SCB::PTR;
        let v = scb.cfsr.read();
        scb.cfsr.write(v);
        v
    };
    LAST.store(cfsr, Relaxed);
    if cfsr & (1 << 17) != 0 {
        // INVSTATE: переход по адресу без бита 0 сбросил T. Сохранённый
        // PC — уже адрес цели; вернём T, и функция выполнится.
        frame.xpsr |= 1 << 24;
    } else {
        // UNDEFINSTR или DIVBYZERO: PC — адрес самой инструкции.
        // SAFETY: PC указывает в код прошивки во flash.
        let first = unsafe { core::ptr::read(frame.pc as *const u16) };
        frame.pc += insn_len(first);
    }
}
// ANCHOR_END: handler

fn report(what: &str) {
    let cfsr = LAST.swap(0, Relaxed);
    hprintln!("{}: CFSR {:#010x}", what, cfsr);
    for f in faults(cfsr, 0) {
        hprintln!("  {}", f.name());
    }
}

// ANCHOR: udiv
/// `udiv` — первая инструкция функции. В QEMU 8.2.2 кадр сбоя DIVBYZERO
/// содержит значения регистров, устаревшие на несколько инструкций того
/// же блока трансляции; у первой инструкции после `bl` предшественников в
/// блоке нет. Пропущенная `udiv` r0 не меняет — функция вернёт делимое.
#[unsafe(naked)]
extern "C" fn udiv_first(num: u32, den: u32) -> u32 {
    naked_asm!("udiv r0, r0, r1", "bx lr");
}
// ANCHOR_END: udiv

#[inline(never)]
fn ping() -> u32 {
    7
}

#[entry]
fn main() -> ! {
    // ANCHOR: enable
    // SAFETY: только включение UsageFault и ловушки деления на ноль.
    unsafe {
        let scb = &*SCB::PTR;
        scb.shcsr.modify(|v| v | 1 << 18); // USGFAULTENA
        scb.ccr.modify(|v| v | 1 << 4); // DIV_0_TRP
    }
    // ANCHOR_END: enable

    // ANCHOR: three
    // SAFETY: udf — намеренный сбой, обработчик её пропустит.
    unsafe { asm!("udf #42") };
    report("udf #42");

    let q = udiv_first(core::hint::black_box(100), core::hint::black_box(0));
    report("udiv на 0");
    hprintln!("  после пропуска udiv в r0 осталось делимое: {}", q);

    let even = ping as *const () as usize & !1;
    // SAFETY: адрес — начало ping, но без бита Thumb: первая инструкция —
    // INVSTATE, обработчик вернёт бит T.
    let f: fn() -> u32 = unsafe { core::mem::transmute(even) };
    let r = f();
    report("переход по чётному адресу");
    hprintln!("  ping вернула {}", r);
    // ANCHOR_END: three

    let unknown = faults(0x0000_4000, 0).count();
    hprintln!("три сбоя пройдены; зарезервированный бит 14 даёт имён: {}", unknown);
    debug::exit(if r == 7 && q == 100 { debug::EXIT_SUCCESS } else { debug::EXIT_FAILURE });
    loop {}
}

#[exception]
unsafe fn HardFault(ef: &cortex_m_rt::ExceptionFrame) -> ! {
    hprintln!("HardFault: pc {:#010x}", ef.pc());
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
