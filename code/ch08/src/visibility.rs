//! Раздел 8.1: модули и видимость.
//!
//! Снаружи крейта (doc-тесты собираются как отдельный крейт):
//!
//! ```compile_fail,E0603
//! // Модуль `bus` объявлен pub(crate): его `pub fn` снаружи не видна.
//! ch08::visibility::station::bus::transfer();
//! ```
//!
//! ```compile_fail,E0616
//! let s = ch08::visibility::station::Sample::new(7, 100);
//! let _ = s.calibrated; // приватное поле
//! ```
//!
//! ```compile_fail,E0616
//! let s = ch08::visibility::station::Sample::new(7, 100);
//! let _ = s.raw; // pub(crate) — для другого крейта то же, что приватное
//! ```
//!
//! ```compile_fail,E0624
//! let mut s = ch08::visibility::station::Sample::new(7, 100);
//! s.reset(); // приватный метод
//! ```
//!
//! ```compile_fail,E0451
//! use ch08::visibility::station::Sample;
//! // Литерал структуры требует доступа ко всем полям.
//! let s = Sample { sensor: 7, raw: 100, calibrated: false };
//! ```
//!
//! А это работает: `pub` на всём пути.
//!
//! ```
//! use ch08::visibility::station::Sample;
//! let mut s = Sample::new(7, 100);
//! s.calibrate();
//! assert_eq!(s.sensor, 7);
//! assert!(s.is_calibrated());
//! ```

// ANCHOR: tree
pub mod station {
    pub struct Sample {
        pub sensor: u16,
        pub(crate) raw: u16,
        calibrated: bool,
    }

    impl Sample {
        pub fn new(sensor: u16, raw: u16) -> Self {
            Sample { sensor, raw, calibrated: false }
        }

        pub fn calibrate(&mut self) {
            self.raw = bus::scale(self.raw);
            self.calibrated = true;
        }

        pub fn is_calibrated(&self) -> bool {
            self.calibrated
        }

        fn reset(&mut self) {
            self.calibrated = false;
        }
    }

    pub(crate) mod bus {
        /// Видна в `station` и ниже.
        pub(super) fn scale(raw: u16) -> u16 {
            raw.saturating_mul(2)
        }

        /// `pub`, но модуль `bus` объявлен `pub(crate)`: видимость
        /// пути — минимум по всем его звеньям.
        pub fn transfer() -> u8 {
            0xA5
        }

        /// Видна внутри `station`: путь в `pub(in …)` абсолютный и должен вести
        /// к модулю-предку. Здесь это то же, что `pub(super)`.
        pub(in crate::visibility::station) fn reset_all(s: &mut super::Sample) {
            s.reset(); // приватный метод `Sample` виден: мы внутри `station`
        }
    }

    pub fn recalibrate(s: &mut Sample) {
        bus::reset_all(s);
        s.calibrate();
    }
}
// ANCHOR_END: tree

// ANCHOR: inside
/// Тот же крейт, другой модуль: `pub(crate)` открыт, приватное закрыто.
pub fn raw_of(s: &station::Sample) -> (u8, u16) {
    (station::bus::transfer(), s.raw)
    // station::bus::scale(s.raw)  — ошибка E0603: функция `scale` приватна
    // s.calibrated                — ошибка E0616: поле `calibrated` приватно
}
// ANCHOR_END: inside

#[cfg(test)]
mod tests {
    use super::station::*;

    #[test]
    fn inside_the_crate() {
        let mut s = Sample::new(7, 100);
        assert_eq!(super::raw_of(&s), (0xA5, 100));
        s.calibrate();
        assert_eq!(s.raw, 200);
        recalibrate(&mut s);
        assert_eq!(s.raw, 400);
        assert!(s.is_calibrated());
    }
}
