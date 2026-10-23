//! Глава 22 на Cortex-M0, M3 и M4F: async без кучи и без ОС. Задача ждёт
//! тиков таймера SysTick; обработчик прерывания отмечает тик и будит
//! исполнитель, а пока будить некому, ядро спит в `wfi`.
//!
//! ```text
//! cd firmware && cargo run --release -p ch22-async
//! ```
#![no_std]
#![no_main]

use ch22::bare::{self, Ticks};
use core::sync::atomic::AtomicBool;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: main
/// Флаг готовности единственного исполнителя и счётчик тиков, который
/// пишет только обработчик SysTick.
static READY: AtomicBool = AtomicBool::new(false);
static TICKS: Ticks = Ticks::new(&READY);

/// Период таймера в тактах ядра: долгий, чтобы печать между тиками успевала.
const RELOAD: u32 = 1_000_000;

#[exception]
fn SysTick() {
    TICKS.tick(); // отметить тик и разбудить исполнитель
}

/// Async-задача: дождаться `n` тиков, печатая каждый.
async fn wait_ticks(n: u32) -> u32 {
    let mut seen = 0;
    for _ in 0..n {
        let t = TICKS.next(&mut seen).await;
        hprintln!("тик {}", t);
    }
    seen
}

#[entry]
fn main() -> ! {
    let mut p = cortex_m::Peripherals::take().unwrap();
    p.SYST.set_clock_source(SystClkSource::Core);
    p.SYST.set_reload(RELOAD);
    p.SYST.clear_current();
    p.SYST.enable_interrupt();
    p.SYST.enable_counter();

    let task = wait_ticks(3);
    let size = core::mem::size_of_val(&task);
    // Future живёт на стеке block_on; спим в wfi под запретом прерываний.
    let run = bare::block_on(&READY, task, bare::cortex_idle);
    p.SYST.disable_counter();

    hprintln!(
        "async на SysTick: тиков {}, опросов {}, снов в wfi {}",
        run.output,
        run.polls,
        run.idles
    );
    hprintln!("автомат задачи: {} байт, куча не нужна", size);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
