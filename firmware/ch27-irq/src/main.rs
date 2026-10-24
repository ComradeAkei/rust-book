//! Глава 27, раздел 27.6: обработчик прерывания и main делят данные.
//! Таймер TIMER0 тикает каждые 10 мс; его обработчику нужен сам таймер,
//! чтобы сбросить запрос, а main — счётчик тиков. Затем две ловушки:
//! обработчик однократного TIMER1, который не сбрасывает запрос, и
//! критическая секция длиннее периода SysTick.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-irq
//! ```
#![no_std]
#![no_main]

use ch27_hal::pac::{self, Interrupt, interrupt};
use ch27_hal::sysctl::{CLOCK_HZ, Sysctl};
use ch27_hal::timer::Timer;
use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use cortex_m::peripheral::{NVIC, syst::SystClkSource};
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::{debug, hprintln};
use critical_section::Mutex;

type Timer0 = Timer<0x4003_0000>;
type Timer1 = Timer<0x4003_1000>;

// ANCHOR: shared
/// Таймер переезжает из main в обработчик: до переезда там `None`.
/// `Mutex` из critical-section отдаёт содержимое только внутри
/// критической секции, `RefCell` — по `&mut` на время заимствования.
static TIMER0: Mutex<RefCell<Option<Timer0>>> = Mutex::new(RefCell::new(None));

/// Счётчик достаточно атомика: одно слово, без критической секции.
static TICKS: AtomicU32 = AtomicU32::new(0);

#[interrupt]
fn TIMER0A() {
    critical_section::with(|cs| {
        if let Some(t) = TIMER0.borrow_ref_mut(cs).as_mut() {
            t.clear();
        }
    });
    TICKS.fetch_add(1, Relaxed);
}
// ANCHOR_END: shared

// ANCHOR: forgot
static TIMER1_ENTRIES: AtomicU32 = AtomicU32::new(0);

/// Ловушка: запрос «досчитал» не сброшен. Он держится, и после выхода
/// NVIC снова входит в обработчик. Предохранитель — маска через 1000
/// входов, иначе это продолжалось бы вечно.
#[interrupt]
fn TIMER1A() {
    if TIMER1_ENTRIES.fetch_add(1, Relaxed) + 1 == 1000 {
        NVIC::mask(Interrupt::TIMER1A);
    }
}
// ANCHOR_END: forgot

// ANCHOR: typo
/// Опечатка в имени: прерывания TIMER0X в PAC нет, и `#[interrupt]`
/// не даёт собрать прошивку (run-demos.sh проверяет текст ошибки).
#[cfg(feature = "typo")]
#[interrupt]
fn TIMER0X() {}
// ANCHOR_END: typo

static SYSTICKS: AtomicU32 = AtomicU32::new(0);

#[exception]
fn SysTick() {
    SYSTICKS.fetch_add(1, Relaxed);
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut cp = pac::CorePeripherals::take().unwrap();
    let mut sc = Sysctl::new(dp.sysctl);

    // ANCHOR: move
    let mut t0 = Timer0::periodic(dp.timer0, CLOCK_HZ / 100, &mut sc);
    t0.listen();
    t0.start();
    critical_section::with(|cs| TIMER0.borrow(cs).replace(Some(t0)));
    // SAFETY: обработчик TIMER0A готов, а таймер уже лежит в TIMER0.
    unsafe { NVIC::unmask(Interrupt::TIMER0A) };
    while TICKS.load(Relaxed) < 10 {
        cortex_m::asm::wfi();
    }
    // ANCHOR_END: move
    hprintln!("TIMER0: 10 тиков по 10 мс, main спал в wfi");

    // ANCHOR: forgot_run
    // Однократный таймер на 1 с: досчитает один раз и встанет.
    let mut t1 = Timer1::one_shot(dp.timer1, CLOCK_HZ, &mut sc);
    t1.listen();
    t1.start();
    // SAFETY: обработчик TIMER1A сам замаскирует прерывание.
    unsafe { NVIC::unmask(Interrupt::TIMER1A) };
    while TIMER1_ENTRIES.load(Relaxed) < 1000 {
        cortex_m::asm::wfi();
    }
    // ANCHOR_END: forgot_run
    hprintln!(
        "TIMER1 досчитал один раз, а обработчик вошёл {} раз; запрос всё ещё висит: {}",
        TIMER1_ENTRIES.load(Relaxed),
        t1.expired()
    );

    // ANCHOR: long_cs
    // SysTick раз в 1 мс, критическая секция на 5 периодов.
    cp.SYST.set_clock_source(SystClkSource::Core);
    cp.SYST.set_reload(CLOCK_HZ / 1000 - 1);
    cp.SYST.clear_current();
    cp.SYST.enable_interrupt();
    cp.SYST.enable_counter();
    let start = SYSTICKS.load(Relaxed);
    let wraps = critical_section::with(|_| {
        // COUNTFLAG ставится при каждом обнулении счётчика и сбрасывается
        // чтением: считаем периоды, не выходя из секции.
        let _ = cp.SYST.has_wrapped();
        let mut wraps = 0;
        while wraps < 5 {
            if cp.SYST.has_wrapped() {
                wraps += 1;
            }
        }
        wraps
    });
    let got = SYSTICKS.load(Relaxed) - start;
    // ANCHOR_END: long_cs
    cp.SYST.disable_counter();
    hprintln!("SysTick: {} периодов в критической секции, обработчик вошёл {} раз", wraps, got);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
