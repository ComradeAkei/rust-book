//! АЦП: последовательность выборки 3 — одна выборка входа 0 на каждый
//! запуск от таймера. QEMU настоящих входов не моделирует и выдаёт
//! псевдослучайные отсчёты около 512 (раздел 27.9).

use crate::pac;
use crate::sysctl::{Gate, Sysctl};

pub struct Adc {
    adc: pac::Adc0,
}

impl Adc {
    // ANCHOR: on_timer
    /// Последовательность 3 запускается таймером с битом TAOTE и по
    /// окончании выборки просит прерывание ADC0SS3.
    pub fn on_timer(adc: pac::Adc0, sc: &mut Sysctl) -> Adc {
        sc.enable(Gate::Adc);
        adc.actss().modify(|_, w| w.ss(3).clear_bit());
        adc.emux().modify(|_, w| w.em(3).timer());
        adc.ssmux3().write(|w| unsafe { w.mux0().bits(0) });
        adc.ssctl3().write(|w| w.end0().set_bit().ie0().set_bit());
        adc.im().modify(|_, w| w.ss(3).set_bit());
        adc.actss().modify(|_, w| w.ss(3).set_bit());
        Adc { adc }
    }
    // ANCHOR_END: on_timer

    /// Отсчёт, если выборка закончилась; запрос прерывания сбрасывается.
    pub fn take(&mut self) -> Option<u16> {
        if self.adc.ris().read().ss(3).bit_is_clear() {
            return None;
        }
        let v = self.adc.ssfifo3().read().data().bits();
        self.adc.isc().write(|w| w.ss(3).set_bit());
        Some(v)
    }

    pub fn free(self) -> pac::Adc0 {
        self.adc
    }
}
