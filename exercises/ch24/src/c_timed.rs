//! Задача C: `#[timed(STATS)]` и всё, на что опирается его раскрытие:
//! счётчик [`Timing`], охранник [`Guard`] и часы [`Clock`].
//!
//! Часы — параметр типа: на хосте [`HostClock`] (наносекунды от
//! `Instant`) и [`FakeClock`] для детерминированных тестов, на Cortex-M —
//! [`SysTickClock`] и, на ARMv7-M и новее, [`DwtCycles`] — счётчик тактов
//! блока DWT. QEMU 8.2 DWT не моделирует: его регистры там читаются нулём.
//!
//! [`Timing`] копит время атомиками `fetch_add`/`fetch_max`, которых нет на
//! ARMv6-M (Cortex-M0): там модуль даёт только часы.

// ANCHOR: clock
/// Источник отсчётов. Отсчёт — 32-битное число, которое переполняется.
/// Часы — тип без данных: охранник хранит `&'static Timing<C>`.
pub trait Clock: 'static {
    fn now() -> u32;
    /// Сколько прошло от `start` до `end`. Счётчик, который считает вверх
    /// и переполняется, — разность с переносом.
    fn elapsed(start: u32, end: u32) -> u32 {
        end.wrapping_sub(start)
    }
}
// ANCHOR_END: clock

#[cfg(target_has_atomic = "32")]
pub use timing::{Guard, Timing};

#[cfg(target_has_atomic = "32")]
mod timing {
    use super::Clock;
    use core::marker::PhantomData;
    use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

    // ANCHOR: timing
    /// Сколько раз вызвана функция, суммарное и наибольшее время одного
    /// вызова — в отсчётах часов `C`.
    pub struct Timing<C> {
        calls: AtomicU32,
        total: AtomicU32,
        max: AtomicU32,
        // fn() -> C: Timing — Sync при любых часах.
        _clock: PhantomData<fn() -> C>,
    }

    impl<C: Clock> Timing<C> {
        pub const fn new() -> Self {
            Self {
                calls: AtomicU32::new(0),
                total: AtomicU32::new(0),
                max: AtomicU32::new(0),
                _clock: PhantomData,
            }
        }

        /// Начать замер. Время запишет `Drop` охранника.
        pub fn start(&'static self) -> Guard<C> {
            Guard { timing: self, start: C::now() }
        }

        pub fn calls(&self) -> u32 {
            self.calls.load(Relaxed)
        }

        pub fn total(&self) -> u32 {
            self.total.load(Relaxed)
        }

        pub fn max(&self) -> u32 {
            self.max.load(Relaxed)
        }

        fn record(&self, t: u32) {
            self.calls.fetch_add(1, Relaxed);
            self.total.fetch_add(t, Relaxed);
            self.max.fetch_max(t, Relaxed);
        }
    }

    impl<C: Clock> Default for Timing<C> {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Охранник замера: живёт, пока выполняется тело функции.
    #[must_use = "охранник, отброшенный сразу (`let _ =`), ничего не замерит"]
    pub struct Guard<C: Clock> {
        timing: &'static Timing<C>,
        start: u32,
    }

    impl<C: Clock> Drop for Guard<C> {
        fn drop(&mut self) {
            self.timing.record(C::elapsed(self.start, C::now()));
        }
    }
    // ANCHOR_END: timing
}

#[cfg(feature = "std")]
pub use host::{FakeClock, HostClock};

#[cfg(feature = "std")]
mod host {
    use super::Clock;
    use std::cell::Cell;
    use std::sync::OnceLock;
    use std::time::Instant;

    /// Наносекунды от первого обращения к часам; переполняются за 4,3 с.
    pub struct HostClock;

    impl Clock for HostClock {
        fn now() -> u32 {
            static START: OnceLock<Instant> = OnceLock::new();
            START.get_or_init(Instant::now).elapsed().as_nanos() as u32
        }
    }

    thread_local! {
        static NOW: Cell<u32> = const { Cell::new(0) };
    }

    /// Часы теста: время идёт, только когда его двигают, и у каждого
    /// потока — своё.
    pub struct FakeClock;

    impl FakeClock {
        pub fn advance(ticks: u32) {
            NOW.set(NOW.get().wrapping_add(ticks));
        }

        pub fn set(ticks: u32) {
            NOW.set(ticks);
        }
    }

    impl Clock for FakeClock {
        fn now() -> u32 {
            NOW.get()
        }
    }
}

#[cfg(target_arch = "arm")]
pub use arm::SysTickClock;

#[cfg(all(target_arch = "arm", target_has_atomic = "32"))]
pub use arm::DwtCycles;

#[cfg(target_arch = "arm")]
mod arm {
    use super::Clock;
    use core::ptr::{read_volatile, write_volatile};

    const SYST_CSR: *mut u32 = 0xE000_E010 as *mut u32;
    const SYST_RVR: *mut u32 = 0xE000_E014 as *mut u32;
    const SYST_CVR: *mut u32 = 0xE000_E018 as *mut u32;

    // ANCHOR: systick
    /// SysTick: 24-битный счётчик, который считает вниз от RVR.
    pub struct SysTickClock;

    impl SysTickClock {
        /// Запустить SysTick от тактовой частоты ядра, с полным периодом.
        ///
        /// # Safety
        /// SysTick больше никто не настраивает.
        pub unsafe fn start() {
            // SAFETY: регистры SysTick есть на любом Cortex-M.
            unsafe {
                write_volatile(SYST_RVR, 0x00FF_FFFF);
                write_volatile(SYST_CVR, 0);
                write_volatile(SYST_CSR, 0b101); // ENABLE | CLKSOURCE
            }
        }
    }

    impl Clock for SysTickClock {
        fn now() -> u32 {
            // SAFETY: чтение CVR побочных эффектов не имеет.
            unsafe { read_volatile(SYST_CVR) }
        }

        /// Счётчик идёт вниз и переполняется через 2^24.
        fn elapsed(start: u32, end: u32) -> u32 {
            start.wrapping_sub(end) & 0x00FF_FFFF
        }
    }
    // ANCHOR_END: systick

    #[cfg(target_has_atomic = "32")]
    const DEMCR: *mut u32 = 0xE000_EDFC as *mut u32;
    #[cfg(target_has_atomic = "32")]
    const DWT_CTRL: *mut u32 = 0xE000_1000 as *mut u32;
    #[cfg(target_has_atomic = "32")]
    const DWT_CYCCNT: *mut u32 = 0xE000_1004 as *mut u32;

    // ANCHOR: dwt
    /// Счётчик тактов DWT (ARMv7-M и новее; на ARMv6-M его нет). Признак
    /// цели — `target_has_atomic = "32"`: у thumbv6m его нет, у thumbv7m и
    /// thumbv7em есть.
    #[cfg(target_has_atomic = "32")]
    pub struct DwtCycles;

    #[cfg(target_has_atomic = "32")]
    impl DwtCycles {
        /// Включить блок трассировки (DEMCR.TRCENA) и счётчик (CYCCNTENA).
        /// Возвращает, включился ли счётчик: в QEMU 8.2 — нет.
        ///
        /// # Safety
        /// DWT больше никто не настраивает (отладчик в том числе).
        pub unsafe fn enable() -> bool {
            // SAFETY: системные регистры отладки ARMv7-M.
            unsafe {
                write_volatile(DEMCR, read_volatile(DEMCR) | 1 << 24);
                write_volatile(DWT_CTRL, read_volatile(DWT_CTRL) | 1);
                read_volatile(DWT_CTRL) & 1 != 0
            }
        }
    }

    #[cfg(target_has_atomic = "32")]
    impl Clock for DwtCycles {
        fn now() -> u32 {
            // SAFETY: чтение CYCCNT побочных эффектов не имеет.
            unsafe { read_volatile(DWT_CYCCNT) }
        }
    }
    // ANCHOR_END: dwt
}
