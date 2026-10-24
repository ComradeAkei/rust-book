//! Глава 27, раздел 27.8: схема раздела 27.6 на RTIC 2. Таймер TIMER0
//! тикает каждые 10 мс в аппаратной задаче, счётчик тиков — общий ресурс,
//! а отчёт печатает программная задача с приоритетом ниже. `lock` в ней —
//! запись в BASEPRI, и прерывание таймера внутри lock ждёт.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-rtic
//! ```
#![no_std]
#![no_main]

use cortex_m_semihosting::debug;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}

// ANCHOR: app
#[rtic::app(device = ch27_pac, dispatchers = [GPIOB])]
mod app {
    use ch27_hal::sysctl::{CLOCK_HZ, Sysctl};
    use ch27_hal::timer::Timer;
    use ch27_pac::Interrupt;
    use cortex_m::peripheral::NVIC;
    use cortex_m::register::{basepri, primask};
    use cortex_m_semihosting::{debug, hprintln};

    type Timer0 = Timer<0x4003_0000>;

    #[shared]
    struct Shared {
        ticks: u32,
    }

    #[local]
    struct Local {
        timer: Timer0,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let mut sc = Sysctl::new(cx.device.sysctl);
        let mut timer = Timer0::periodic(cx.device.timer0, CLOCK_HZ / 100, &mut sc);
        timer.listen();
        timer.start();
        hprintln!(
            "init: PRIMASK {}, BASEPRI {:#04x}",
            u8::from(primask::read().is_inactive()),
            basepri::read()
        );
        (Shared { ticks: 0 }, Local { timer })
    }
    // ANCHOR_END: app

    // ANCHOR: tick
    /// Аппаратная задача: RTIC сам включит TIMER0A в NVIC и выставит ему
    /// приоритет 2. Таймер — локальный ресурс: он только здесь.
    #[task(binds = TIMER0A, priority = 2, local = [timer, n: u32 = 0], shared = [ticks])]
    fn tick(mut cx: tick::Context) {
        cx.local.timer.clear();
        cx.shared.ticks.lock(|t| *t += 1);
        *cx.local.n += 1;
        if *cx.local.n == 5 {
            // Ошибка — только если report ещё не завершилась.
            report::spawn(*cx.local.n).ok();
        }
    }
    // ANCHOR_END: tick

    // ANCHOR: report
    /// Программная задача с приоритетом 1. Её выполняет обработчик GPIOB
    /// из списка `dispatchers`: одно свободное прерывание на каждый
    /// уровень приоритета программных задач.
    #[task(priority = 1, shared = [ticks])]
    async fn report(mut cx: report::Context, seen: u32) {
        hprintln!("report: запущена на тике {}, BASEPRI {:#04x}", seen, basepri::read());
        let (inside, before, after) = cx.shared.ticks.lock(|t| {
            let before = *t;
            // Ждём, пока таймер досчитает: запрос висит, но tick не входит.
            while !NVIC::is_pending(Interrupt::TIMER0A) {}
            (basepri::read(), before, *t)
        });
        hprintln!(
            "в lock: BASEPRI {:#04x}, тиков до {} и после {}, TIMER0A ждал",
            inside,
            before,
            after
        );
        let now = cx.shared.ticks.lock(|t| *t);
        hprintln!("после lock: BASEPRI {:#04x}, тиков {}", basepri::read(), now);
        debug::exit(debug::EXIT_SUCCESS);
    }
    // ANCHOR_END: report
}
