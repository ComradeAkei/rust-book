//! Раздел 25.4: `static` с внутренней изменяемостью.
//!
//! В редакции 2024 ссылка на `static mut` — ошибка (запрещающая проверка
//! `static_mut_refs`), даже неявная, через вызов метода:
//!
//! ```compile_fail
//! pub static mut SAMPLES: [u16; 8] = [0; 8];
//!
//! pub fn count() -> usize {
//!     unsafe { SAMPLES.len() } // creating a shared reference to mutable static
//! }
//! ```
//!
//! Обращение к самому месту — чтение, запись, индекс — ссылку не создаёт и
//! разрешено в `unsafe`. Сырой указатель `&raw mut` берётся даже без
//! `unsafe`: опасно разыменование, а не адрес.
//!
//! ```
//! pub static mut SAMPLES: [u16; 8] = [0; 8];
//! pub static mut N: usize = 0;
//!
//! pub fn push(v: u16) {
//!     // SAFETY: один поток, обработчиков прерываний, которые трогают
//!     // SAMPLES и N, нет.
//!     unsafe {
//!         SAMPLES[N % 8] = v;
//!         N += 1;
//!     }
//! }
//!
//! pub fn samples() -> *mut u16 {
//!     (&raw mut SAMPLES).cast()
//! }
//!
//! push(7);
//! assert_eq!(unsafe { *samples() }, 7);
//! ```

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// ANCHOR: atomics
/// Счётчик тиков SysTick: пишет обработчик прерывания, читает основной
/// цикл. Атомик даёт внутреннюю изменяемость без `unsafe` и без `mut`.
pub static TICKS: AtomicU32 = AtomicU32::new(0);

/// Флаг «прошивка инициализирована»: ставится один раз.
pub static READY: AtomicBool = AtomicBool::new(false);

/// `fetch_add` — атомарное чтение-изменение-запись. На Cortex-M0 таких
/// инструкций нет (раздел 21.4), и функции там нет тоже.
#[cfg(target_has_atomic = "32")]
pub fn on_systick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn ticks() -> u32 {
    TICKS.load(Ordering::Relaxed)
}
// ANCHOR_END: atomics

// ANCHOR: once-flag
/// Ресурс, который можно забрать один раз: `true` только у первого
/// вызова. Так устроен `Peripherals::take()` из cortex-m (глава 27), но
/// без критической секции — на compare_exchange.
pub static TAKEN: AtomicBool = AtomicBool::new(false);

#[cfg(target_has_atomic = "32")]
pub fn take_once() -> bool {
    TAKEN.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_ok()
}
// ANCHOR_END: once-flag

#[cfg(feature = "std")]
pub mod hosted {
    use std::sync::{LazyLock, Mutex};

    // ANCHOR: mutex
    /// Настройки станции. `Mutex::new` — `const fn` (Rust 1.63), и мьютекс
    /// готов до `main`, как `PTHREAD_MUTEX_INITIALIZER` в C: никакого
    /// кода инициализации, только данные.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Config {
        pub period_ms: u32,
        pub channels: u8,
    }

    impl Config {
        pub const DEFAULT: Config = Config { period_ms: 1000, channels: 4 };
    }

    pub static CONFIG: Mutex<Config> = Mutex::new(Config::DEFAULT);

    pub fn set_period(ms: u32) {
        CONFIG.lock().unwrap().period_ms = ms;
    }
    // ANCHOR_END: mutex

    // ANCHOR: lazy
    /// Таблица, которую нельзя посчитать при компиляции: её параметр
    /// приходит из окружения при запуске. `LazyLock` вычислит её при
    /// первом обращении, а каждое обращение проверит, готова ли она.
    pub static SCALE: LazyLock<[u32; 256]> = LazyLock::new(|| {
        let k = std::env::var("STATION_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
        std::array::from_fn(|i| i as u32 * k)
    });

    /// Та же таблица с известным при сборке множителем — просто данные
    /// в `.rodata`, без проверки и без кода инициализации.
    pub static SCALE3: [u32; 256] = {
        let mut t = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            t[i] = i as u32 * 3;
            i += 1;
        }
        t
    };
    // ANCHOR_END: lazy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_and_flags() {
        let before = ticks();
        on_systick();
        on_systick();
        assert!(ticks() >= before + 2);
        READY.store(true, Ordering::Release);
        assert!(READY.load(Ordering::Acquire));
    }

    /// Из восьми потоков ресурс достаётся ровно одному.
    #[test]
    fn take_once_has_one_winner() {
        let won = std::thread::scope(|s| {
            let hs: Vec<_> = (0..8).map(|_| s.spawn(take_once)).collect();
            hs.into_iter().map(|h| h.join().unwrap()).filter(|&w| w).count()
        });
        assert_eq!(won, 1);
        assert!(!take_once());
    }

    #[test]
    fn hosted_statics() {
        use hosted::*;
        set_period(250);
        assert_eq!(CONFIG.lock().unwrap().period_ms, 250);
        assert_eq!(CONFIG.lock().unwrap().channels, Config::DEFAULT.channels);
        assert_eq!(SCALE3[100], 300);
        // Без переменной окружения множитель тот же, 3.
        if std::env::var_os("STATION_SCALE").is_none() {
            assert_eq!(SCALE[..], SCALE3[..]);
        }
    }

    /// Наблюдение, x86-64 Linux, rustc 1.97: `LazyLock` хранит таблицу и
    /// состояние `Once` — на 8 байт больше самой таблицы. `Mutex<u32>` —
    /// 4 байта futex, флаг отравления и сам `u32`.
    #[test]
    fn observed_sizes() {
        use std::sync::{LazyLock, Mutex};
        assert_eq!(size_of::<LazyLock<[u32; 256]>>(), 1032);
        assert_eq!(size_of::<Mutex<u32>>(), 12);
    }
}
