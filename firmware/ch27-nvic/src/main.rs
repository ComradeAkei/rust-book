//! Глава 27, раздел 27.2: кто кого вытесняет. Прерывания портов GPIOA,
//! GPIOB и GPIOC здесь — программные: их ставят в ожидание записью в
//! NVIC, сами порты не настроены. Каждый обработчик пишет свой номер в
//! журнал, и по журналу видно, в каком порядке они выполнялись.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-nvic
//! ```
#![no_std]
#![no_main]

use ch27::prio;
use ch27_pac::{self as pac, Interrupt, NVIC_PRIO_BITS, interrupt};
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering::Relaxed};
use cortex_m::peripheral::NVIC;
use cortex_m::register::basepri;
use cortex_m_rt::{ExceptionFrame, entry, exception};
use cortex_m_semihosting::{debug, hprint, hprintln};

// ANCHOR: log
/// Журнал: номер события по порядку. Пишут и main, и обработчики, поэтому
/// всё атомарное; на одном ядре порядок записей — порядок выполнения.
static LOG: [AtomicU8; 16] = [const { AtomicU8::new(0) }; 16];
static LEN: AtomicUsize = AtomicUsize::new(0);

fn log(event: u8) {
    let i = LEN.fetch_add(1, Relaxed);
    if i < LOG.len() {
        LOG[i].store(event, Relaxed);
    }
}

fn dump(title: &str) {
    hprint!("{}:", title);
    for e in &LOG[..LEN.swap(0, Relaxed).min(LOG.len())] {
        hprint!(" {}", char::from(e.load(Relaxed)));
    }
    hprintln!();
}
// ANCHOR_END: log

// ANCHOR: handlers
/// Средний приоритет. Изнутри ставит в ожидание оба других.
#[interrupt]
fn GPIOA() {
    log(b'A');
    NVIC::pend(Interrupt::GPIOB);
    NVIC::pend(Interrupt::GPIOC);
    log(b'a');
}

/// Высокий приоритет.
#[interrupt]
fn GPIOB() {
    log(b'B');
}

/// Низкий приоритет.
#[interrupt]
fn GPIOC() {
    log(b'C');
}
// ANCHOR_END: handlers

#[entry]
fn main() -> ! {
    let mut cp = pac::CorePeripherals::take().unwrap();

    // ANCHOR: bits
    // Сколько старших битов приоритета хранит NVIC: записать 0xFF и
    // прочитать обратно.
    // SAFETY: прерывание ещё не разрешено, его приоритет ни на что не влияет.
    unsafe { cp.NVIC.set_priority(Interrupt::GPIOA, 0xFF) };
    let stored = NVIC::get_priority(Interrupt::GPIOA);
    hprintln!(
        "приоритет после записи 0xff: {:#04x}, битов {}, в SVD — {}",
        stored,
        stored.count_ones(),
        NVIC_PRIO_BITS
    );
    // ANCHOR_END: bits

    // ANCHOR: setup
    // Уровни в трёх битах, как у LM3S6965: 2 — 0x40, 4 — 0x80, 6 — 0xC0.
    // Меньшее число — важнее.
    let level = |l| prio::encode(l, NVIC_PRIO_BITS);
    // SAFETY: ни один обработчик не держит критической секции, которую
    // смена приоритета могла бы сломать.
    unsafe {
        cp.NVIC.set_priority(Interrupt::GPIOB, level(2));
        cp.NVIC.set_priority(Interrupt::GPIOA, level(4));
        cp.NVIC.set_priority(Interrupt::GPIOC, level(6));
        NVIC::unmask(Interrupt::GPIOA);
        NVIC::unmask(Interrupt::GPIOB);
        NVIC::unmask(Interrupt::GPIOC);
    }
    // ANCHOR_END: setup

    // ANCHOR: run
    log(b'm');
    NVIC::pend(Interrupt::GPIOA);
    log(b'M');
    dump("вытеснение");

    // BASEPRI = 0x80: ждут все с приоритетом 0x80 и ниже — A и C.
    // SAFETY: запись BASEPRI небезопасна, потому что снятие маски ломает
    // чужую критическую секцию на ней (раздел 27.8). Здесь секций нет.
    unsafe { basepri::write(level(4)) };
    NVIC::pend(Interrupt::GPIOA);
    NVIC::pend(Interrupt::GPIOB);
    NVIC::pend(Interrupt::GPIOC);
    log(b'm');
    let waiting = [Interrupt::GPIOA, Interrupt::GPIOB, Interrupt::GPIOC].map(NVIC::is_pending);
    // SAFETY: см. выше.
    unsafe { basepri::write(0) };
    log(b'M');
    dump("BASEPRI 0x80");
    hprintln!("ждали под BASEPRI: A {}, B {}, C {}", waiting[0], waiting[1], waiting[2]);
    // ANCHOR_END: run

    // ANCHOR: svc
    // SVCall — синхронное исключение: оно должно выполниться сразу. Если
    // маска не пускает его, процессор поднимает HardFault.
    // SAFETY: приоритет SVCall меняем, пока ни одного SVCall нет.
    unsafe { cp.SCB.set_priority(cortex_m::peripheral::scb::SystemHandler::SVCall, level(4)) };
    // SAFETY: см. выше.
    unsafe { basepri::write(level(4)) };
    hprintln!("svc под BASEPRI 0x80 при приоритете SVCall 0x80…");
    // SAFETY: до обработчика SVCall дело не дойдёт: это и проверяем.
    unsafe { core::arch::asm!("svc #0") };
    hprintln!("svc выполнился — так быть не должно");
    debug::exit(debug::EXIT_FAILURE);
    // ANCHOR_END: svc
    loop {}
}

#[exception]
fn SVCall() {
    hprintln!("SVCall");
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    // SAFETY: HFSR — регистр только для чтения состояния сбоя.
    let hfsr = unsafe { (*cortex_m::peripheral::SCB::PTR).hfsr.read() };
    hprintln!("HardFault: HFSR {:#010x}, FORCED {}, pc {:#010x}", hfsr, hfsr >> 30 & 1, ef.pc());
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
