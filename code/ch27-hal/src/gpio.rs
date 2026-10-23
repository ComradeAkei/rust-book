//! Выводы порта с режимом в типе. Вывод — тип нулевого размера: адрес
//! порта и номер вывода — параметры-константы, режим — тип-маркер.
//! Сменить режим — значит отдать вывод и получить его в новом типе.

use crate::pac;
use crate::sysctl::{Gate, Sysctl};
use core::convert::Infallible;
use core::marker::PhantomData;
use embedded_hal::digital::{ErrorType, InputPin, OutputPin, StatefulOutputPin};

// ANCHOR: modes
/// Вход. После сброса выводы — входы.
pub struct Input;
/// Выход.
pub struct Output;
/// Выводом управляет периферия. У каждого вывода LM3S6965 одна
/// альтернативная функция: PA0 — приём UART0, PA1 — передача.
pub struct Alternate;
// ANCHOR_END: modes

// ANCHOR: pin
/// Вывод `N` порта по адресу `PORT` в режиме `MODE`. Полей, кроме
/// маркера, нет: размер — ноль, всё, что знает тип, известно при
/// компиляции.
pub struct Pin<const PORT: usize, const N: u8, MODE> {
    _mode: PhantomData<MODE>,
}
// ANCHOR_END: pin

impl<const PORT: usize, const N: u8, MODE> Pin<PORT, N, MODE> {
    const fn new() -> Self {
        Pin { _mode: PhantomData }
    }

    fn regs() -> &'static pac::gpioa::RegisterBlock {
        // SAFETY: PORT — адрес одного из портов (типы ниже создаются только
        // для них), блок регистров живёт всё время работы программы.
        unsafe { &*(PORT as *const pac::gpioa::RegisterBlock) }
    }

    // ANCHOR: bitband
    /// Бит `N` регистра порта — одной записью в слово-псевдоним bit-band.
    /// Чтения-изменения-записи нет: другой вывод того же порта, который
    /// меняет режим в прерывании, свой бит не потеряет.
    fn put(reg: *mut u32, on: bool) {
        let alias = ch27::bitband::alias(reg as u32, N);
        // SAFETY: регистр порта лежит в области bit-band периферии, его
        // псевдоним — тоже регистр, запись одного бита не трогает соседей.
        unsafe { core::ptr::write_volatile(alias as *mut u32, on as u32) }
    }
    // ANCHOR_END: bitband

    // ANCHOR: into
    pub fn into_output(self) -> Pin<PORT, N, Output> {
        let r = Self::regs();
        Self::put(r.afsel().as_ptr(), false);
        Self::put(r.dir().as_ptr(), true);
        Self::put(r.den().as_ptr(), true);
        Pin::new()
    }

    pub fn into_input(self) -> Pin<PORT, N, Input> {
        let r = Self::regs();
        Self::put(r.afsel().as_ptr(), false);
        Self::put(r.dir().as_ptr(), false);
        Self::put(r.den().as_ptr(), true);
        Pin::new()
    }

    pub fn into_alternate(self) -> Pin<PORT, N, Alternate> {
        let r = Self::regs();
        Self::put(r.afsel().as_ptr(), true);
        Self::put(r.den().as_ptr(), true);
        Pin::new()
    }
    // ANCHOR_END: into
}

impl<const PORT: usize, const N: u8, MODE> ErrorType for Pin<PORT, N, MODE> {
    type Error = Infallible;
}

// ANCHOR: output
impl<const PORT: usize, const N: u8> OutputPin for Pin<PORT, N, Output> {
    fn set_high(&mut self) -> Result<(), Infallible> {
        // Биты 2–9 адреса — маска: запись по адресу маски 1 << N меняет
        // только вывод N, остальные выводы порта она не трогает.
        Self::regs().data(1 << N).write(|w| w.p(N).set_bit());
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Infallible> {
        Self::regs().data(1 << N).write(|w| w.p(N).clear_bit());
        Ok(())
    }
}

impl<const PORT: usize, const N: u8> StatefulOutputPin for Pin<PORT, N, Output> {
    fn is_set_high(&mut self) -> Result<bool, Infallible> {
        Ok(Self::regs().data(1 << N).read().p(N).bit_is_set())
    }

    fn is_set_low(&mut self) -> Result<bool, Infallible> {
        self.is_set_high().map(|h| !h)
    }
}

impl<const PORT: usize, const N: u8> InputPin for Pin<PORT, N, Input> {
    fn is_high(&mut self) -> Result<bool, Infallible> {
        Ok(Self::regs().data(1 << N).read().p(N).bit_is_set())
    }

    fn is_low(&mut self) -> Result<bool, Infallible> {
        self.is_high().map(|h| !h)
    }
}
// ANCHOR_END: output

/// Порт делится на восемь выводов, каждый — отдельное значение.
pub trait GpioExt {
    type Parts;
    fn split(self, sc: &mut Sysctl) -> Self::Parts;
}

// ANCHOR: port
macro_rules! port {
    ($Periph:ident, $port:ident, $BASE:ident = $base:literal, $gate:literal,
     [$($Pin:ident $pin:ident $n:literal),*]) => {
        /// Адрес блока регистров порта.
        pub const $BASE: usize = $base;

        $(pub type $Pin<MODE = Input> = Pin<$BASE, $n, MODE>;)*

        pub mod $port {
            use super::*;

            /// Выводы порта после сброса — входы.
            pub struct Parts {
                $(pub $pin: $Pin,)*
            }

            impl GpioExt for pac::$Periph {
                type Parts = Parts;

                /// Включает такты порта и раздаёт выводы. Значение порта
                /// поглощается: второго `split` быть не может.
                fn split(self, sc: &mut Sysctl) -> Parts {
                    sc.enable(Gate::Gpio($gate));
                    Parts { $($pin: Pin::new(),)* }
                }
            }
        }
    };
}
// ANCHOR_END: port

port!(Gpioa, gpioa, GPIOA = 0x4000_4000, 0,
      [PA0 pa0 0, PA1 pa1 1, PA2 pa2 2, PA3 pa3 3, PA4 pa4 4, PA5 pa5 5, PA6 pa6 6, PA7 pa7 7]);
port!(Gpiob, gpiob, GPIOB = 0x4000_5000, 1,
      [PB0 pb0 0, PB1 pb1 1, PB2 pb2 2, PB3 pb3 3, PB4 pb4 4, PB5 pb5 5, PB6 pb6 6, PB7 pb7 7]);
port!(Gpioc, gpioc, GPIOC = 0x4000_6000, 2,
      [PC0 pc0 0, PC1 pc1 1, PC2 pc2 2, PC3 pc3 3, PC4 pc4 4, PC5 pc5 5, PC6 pc6 6, PC7 pc7 7]);
port!(Gpiod, gpiod, GPIOD = 0x4000_7000, 3,
      [PD0 pd0 0, PD1 pd1 1, PD2 pd2 2, PD3 pd3 3, PD4 pd4 4, PD5 pd5 5, PD6 pd6 6, PD7 pd7 7]);
port!(Gpioe, gpioe, GPIOE = 0x4002_4000, 4,
      [PE0 pe0 0, PE1 pe1 1, PE2 pe2 2, PE3 pe3 3, PE4 pe4 4, PE5 pe5 5, PE6 pe6 6, PE7 pe7 7]);
port!(Gpiof, gpiof, GPIOF = 0x4002_5000, 5,
      [PF0 pf0 0, PF1 pf1 1, PF2 pf2 2, PF3 pf3 3, PF4 pf4 4, PF5 pf5 5, PF6 pf6 6, PF7 pf7 7]);
port!(Gpiog, gpiog, GPIOG = 0x4002_6000, 6,
      [PG0 pg0 0, PG1 pg1 1, PG2 pg2 2, PG3 pg3 3, PG4 pg4 4, PG5 pg5 5, PG6 pg6 6, PG7 pg7 7]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_addresses_match_pac() {
        assert_eq!(GPIOA, pac::Gpioa::PTR as usize);
        assert_eq!(GPIOD, pac::Gpiod::PTR as usize);
        assert_eq!(GPIOE, pac::Gpioe::PTR as usize);
        assert_eq!(GPIOG, pac::Gpiog::PTR as usize);
    }

    #[test]
    fn pins_are_zero_sized() {
        assert_eq!(size_of::<PF0<Output>>(), 0);
        assert_eq!(size_of::<gpiof::Parts>(), 0);
    }
}
