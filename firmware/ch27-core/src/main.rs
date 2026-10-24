//! Глава 27, раздел 27.1: что видит процессор в режимах Thread и Handler.
//! Программа печатает свои IPSR, CONTROL и указатели стека, делает
//! системный вызов `svc #1` (сложить два числа) и `svc #2` (отчёт о
//! кадре), затем переводит код в режиме Thread на отдельный стек PSP и
//! повторяет то же самое.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-core
//! ```
#![no_std]
#![no_main]

use ch27::exc::{ExcReturn, Exception, Frame, svc_number};
use core::arch::{asm, naked_asm};
use cortex_m::register::{control, msp, psp};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

fn ipsr() -> u32 {
    let r: u32;
    // SAFETY: чтение IPSR ничего не меняет.
    unsafe { asm!("mrs {}, IPSR", out(reg) r, options(nomem, nostack, preserves_flags)) };
    r
}

fn report(stage: &str) {
    hprintln!(
        "{}: {:?}, IPSR {}, CONTROL {:#x}, MSP {:#010x}, PSP {:#010x}",
        stage,
        Exception::from_ipsr(ipsr()),
        ipsr(),
        control::read().bits(),
        msp::read(),
        psp::read()
    );
}

// ANCHOR: syscalls
/// Системный вызов 1: `a + b`. Аргументы и ответ — в r0 и r1, как у
/// обычной функции по AAPCS; номер вызова — в самой инструкции.
fn sys_add(a: u32, b: u32) -> u32 {
    let r;
    // SAFETY: обработчик SVCall меняет только сохранённый r0.
    unsafe { asm!("svc #1", inout("r0") a => r, in("r1") b) };
    r
}

/// Системный вызов 2: отчёт обработчика о кадре и EXC_RETURN.
fn sys_report() {
    // SAFETY: обработчик только печатает.
    unsafe { asm!("svc #2") };
}
// ANCHOR_END: syscalls

// ANCHOR: handler
/// Вход в SVCall. Процессор уже сохранил r0–r3, r12, LR, PC и xPSR на
/// стек прерванного кода, а в LR положил EXC_RETURN. Бит 2 EXC_RETURN
/// говорит, на каком стеке кадр: 0 — MSP, 1 — PSP. Обработчику на Rust
/// достаются адрес кадра и EXC_RETURN; `b`, а не `bl`, оставляет LR
/// нетронутым, и возврат из `svc_rust` — это выход из исключения.
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn SVCall() {
    naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "mov r1, lr",
        "b {rust}",
        rust = sym svc_rust,
    );
}

extern "C" fn svc_rust(frame: &mut Frame, exc_return: u32) {
    // Сохранённый PC — адрес инструкции после svc; сама svc на 2 байта
    // раньше, номер — её младший байт.
    // SAFETY: PC указывает во flash прошивки, в код, который только что
    // выполнялся.
    let insn = unsafe { core::ptr::read((frame.pc - 2) as *const u16) };
    match svc_number(insn) {
        Some(1) => frame.r0 = frame.r0.wrapping_add(frame.r1),
        Some(2) => {
            let at = frame as *const Frame as u32;
            hprintln!(
                "  SVCall: {:?}, IPSR {}, EXC_RETURN {:#010x} = {:?}",
                Exception::from_ipsr(ipsr()),
                ipsr(),
                exc_return,
                ExcReturn::decode(exc_return).unwrap()
            );
            hprintln!(
                "  кадр по адресу {:#010x}: pc {:#010x}, lr {:#010x}, xpsr {:#010x}, SP до входа {:#010x}",
                at,
                frame.pc,
                frame.lr,
                frame.xpsr,
                frame.sp_before(at)
            );
        }
        _ => hprintln!("  SVCall: неизвестный вызов {:#06x}", insn),
    }
}
// ANCHOR_END: handler

// ANCHOR: psp
/// Отдельный стек для кода в режиме Thread: 1 КиБ, выравнивание 8.
static mut PSTACK: [u64; 128] = [0; 128];

/// Переводит режим Thread на PSP и продолжает в `next`. Менять SP посреди
/// функции на Rust нельзя: её локальные переменные и адрес возврата
/// остались на старом стеке. Поэтому переход — на ассемблере, и
/// продолжение — новая функция, которая не возвращается.
#[unsafe(naked)]
unsafe extern "C" fn switch_to_psp(top: *mut u64, next: extern "C" fn() -> !) -> ! {
    naked_asm!(
        "msr psp, r0",
        "mrs r2, control",
        "orr r2, r2, #2", // SPSEL = 1: Thread работает на PSP
        "msr control, r2",
        "isb", // следующие инструкции уже видят новый SP
        "bx r1",
    );
}
// ANCHOR_END: psp

#[entry]
fn main() -> ! {
    report("main");
    hprintln!("svc #1 (20, 22) = {}", sys_add(20, 22));
    sys_report();
    // SAFETY: PSTACK больше никто не трогает; вершина — конец массива.
    unsafe { switch_to_psp((&raw mut PSTACK).cast::<u64>().add(128), on_psp) }
}

extern "C" fn on_psp() -> ! {
    report("на PSP");
    hprintln!("svc #1 (40, 2) = {}", sys_add(40, 2));
    sys_report();
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
