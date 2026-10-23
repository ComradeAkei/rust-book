//! Задержка по SysTick: трейт `DelayNs` из embedded-hal 1.0. Драйверы,
//! которым нужно «подождать 10 мкс», берут любой `DelayNs` и не знают,
//! каким таймером он сделан.

use crate::pac::SYST;
use crate::sysctl::CLOCK_HZ;
use cortex_m::peripheral::syst::SystClkSource;
use embedded_hal::delay::DelayNs;

pub struct Delay {
    syst: SYST,
}

impl Delay {
    /// SysTick считает такты ядра от 2²⁴ − 1 вниз, по кругу.
    pub fn new(mut syst: SYST) -> Delay {
        syst.set_clock_source(SystClkSource::Core);
        syst.set_reload(0x00FF_FFFF);
        syst.clear_current();
        syst.enable_counter();
        Delay { syst }
    }

    pub fn free(self) -> SYST {
        self.syst
    }
}

// ANCHOR: delay
impl DelayNs for Delay {
    fn delay_ns(&mut self, ns: u32) {
        let mut left = (u64::from(ns) * u64::from(CLOCK_HZ)).div_ceil(1_000_000_000);
        let mut last = SYST::get_current();
        while left > 0 {
            let now = SYST::get_current();
            // Счётчик 24-битный и идёт вниз: прошло last − now по модулю 2²⁴.
            let passed = last.wrapping_sub(now) & 0x00FF_FFFF;
            left = left.saturating_sub(u64::from(passed));
            last = now;
        }
    }
}
// ANCHOR_END: delay
