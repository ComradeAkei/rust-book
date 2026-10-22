//! Раздел 9.2: ассоциированные типы и константы.
//!
//! Ассоциированный тип — выход реализации: у каждого типа он один, и
//! компилятор выводит его из типа получателя.
//!
//! ```
//! use ch09::assoc::{Adc, Source};
//! let mut adc = Adc { next: 7 };
//! let x = adc.sample(); // тип x — u16, он задан реализацией
//! assert_eq!(x, 8u16);
//! ```
//!
//! Параметр трейта — вход: реализаций может быть несколько, и без
//! подсказки компилятор не знает, какую взять.
//!
//! ```compile_fail,E0283
//! use ch09::assoc::{Adc, SourceOf};
//! let mut adc = Adc { next: 7 };
//! let x = adc.get(); // u16 или u32?
//! ```
//!
//! ```
//! use ch09::assoc::{Adc, SourceOf};
//! let mut adc = Adc { next: 7 };
//! let x: u32 = adc.get();
//! assert_eq!(x, 8 << 4);
//! ```
//!
//! Проверка выравнивания в `Reg` — константа, которую компилятор
//! вычисляет для каждого регистра, где `Reg` использован. Невыровненный
//! адрес — ошибка компиляции, но только там, где до него дошло дело.
//!
//! ```compile_fail,E0080
//! use ch09::assoc::{Reg, Register};
//! struct Bad;
//! impl Register for Bad {
//!     const ADDR: usize = 0xE000_E011;
//!     const NAME: &'static str = "BAD";
//! }
//! unsafe { Reg::<Bad>::write(0) }
//! ```
//!
//! ```
//! use ch09::assoc::Register;
//! struct Bad;
//! impl Register for Bad {
//!     const ADDR: usize = 0xE000_E011;
//!     const NAME: &'static str = "BAD";
//! }
//! assert_eq!(Bad::ADDR % 4, 1); // объявить можно, обратиться через Reg — нет
//! ```

use core::marker::PhantomData;

// ANCHOR: source
/// Источник отсчётов, тип отсчёта задаёт реализация.
pub trait Source {
    type Sample: Copy + Default;

    fn sample(&mut self) -> Self::Sample;
}

/// 12-разрядный АЦП: отсчёт помещается в `u16`.
pub struct Adc {
    pub next: u16,
}

impl Source for Adc {
    type Sample = u16;

    fn sample(&mut self) -> u16 {
        self.next = self.next.wrapping_add(1) & 0x0fff;
        self.next
    }
}

/// Акселерометр: три оси за раз.
pub struct Accel {
    pub z: i16,
}

impl Source for Accel {
    type Sample = [i16; 3];

    fn sample(&mut self) -> [i16; 3] {
        [0, 0, self.z]
    }
}

/// Заполнить буфер отсчётами. Тип элементов буфера следует из `S`.
pub fn fill<S: Source>(s: &mut S, buf: &mut [S::Sample]) {
    for x in buf {
        *x = s.sample();
    }
}
// ANCHOR_END: source

// ANCHOR: source-of
/// То же через параметр трейта: у `Adc` две реализации, и какой тип
/// вернёт `get`, решает вызывающий.
pub trait SourceOf<T> {
    fn get(&mut self) -> T;
}

impl SourceOf<u16> for Adc {
    fn get(&mut self) -> u16 {
        self.sample()
    }
}

/// Отсчёт, выровненный влево в 16 битах и расширенный до 32.
impl SourceOf<u32> for Adc {
    fn get(&mut self) -> u32 {
        u32::from(self.sample()) << 4
    }
}
// ANCHOR_END: source-of

// ANCHOR: register
/// Регистр периферии: адрес известен при компиляции.
pub trait Register {
    const ADDR: usize;
    const NAME: &'static str;
}

/// Регистры системного таймера SysTick, одинаковые во всех Cortex-M
/// (ARMv7-M ARM, B3.3).
pub struct SystCsr;
pub struct SystRvr;
pub struct SystCvr;

impl Register for SystCsr {
    const ADDR: usize = 0xE000_E010;
    const NAME: &'static str = "SYST_CSR";
}

impl Register for SystRvr {
    const ADDR: usize = 0xE000_E014;
    const NAME: &'static str = "SYST_RVR";
}

impl Register for SystCvr {
    const ADDR: usize = 0xE000_E018;
    const NAME: &'static str = "SYST_CVR";
}

/// Доступ к регистру `R`. Сам `Reg` — тип нулевого размера: всё, что он
/// знает, записано в константах `R`.
pub struct Reg<R>(PhantomData<R>);

impl<R: Register> Reg<R> {
    /// Вычисляется для каждого `R`, с которым `Reg` использован.
    const ALIGNED: () = assert!(R::ADDR % 4 == 0, "адрес регистра не выровнен на 4");

    /// # Safety
    ///
    /// По адресу `R::ADDR` должен быть регистр, который можно читать.
    #[inline(always)]
    pub unsafe fn read() -> u32 {
        let () = Self::ALIGNED;
        // SAFETY: по контракту функции.
        unsafe { (R::ADDR as *const u32).read_volatile() }
    }

    /// # Safety
    ///
    /// По адресу `R::ADDR` должен быть регистр, запись в который не
    /// нарушает инвариантов остальной программы.
    #[inline(always)]
    pub unsafe fn write(value: u32) {
        let () = Self::ALIGNED;
        // SAFETY: по контракту функции.
        unsafe { (R::ADDR as *mut u32).write_volatile(value) }
    }
}
// ANCHOR_END: register

// ANCHOR: systick
/// CSR: ENABLE (бит 0) и CLKSOURCE (бит 2) — такт процессора.
pub const SYST_CSR_START: u32 = 0b101;
/// CSR: COUNTFLAG (бит 16) — счётчик дошёл до нуля с прошлого чтения.
pub const SYST_CSR_COUNTFLAG: u32 = 1 << 16;

/// Запустить SysTick с периодом `reload + 1` тактов.
///
/// # Safety
///
/// Только на Cortex-M, и SysTick не должен использовать никто другой.
#[unsafe(no_mangle)]
pub unsafe fn ch09_systick_start(reload: u32) {
    // SAFETY: регистры SysTick есть в каждом Cortex-M, по контракту
    // функции таймер наш.
    unsafe {
        Reg::<SystRvr>::write(reload);
        Reg::<SystCvr>::write(0);
        Reg::<SystCsr>::write(SYST_CSR_START);
    }
}
// ANCHOR_END: systick

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn associated_type_follows_impl() {
        let mut adc = Adc { next: 0x0ffe };
        let mut buf = [0u16; 3];
        fill(&mut adc, &mut buf);
        assert_eq!(buf, [0x0fff, 0, 1]);

        let mut acc = Accel { z: 1000 };
        let mut buf = [[0i16; 3]; 2];
        fill(&mut acc, &mut buf);
        assert_eq!(buf, [[0, 0, 1000]; 2]);
    }

    #[test]
    fn generic_parameter_needs_annotation() {
        let mut adc = Adc { next: 0 };
        let a: u16 = adc.get();
        let b = SourceOf::<u32>::get(&mut adc);
        assert_eq!((a, b), (1, 2 << 4));
    }

    #[test]
    fn registers() {
        assert_eq!(
            [SystCsr::ADDR, SystRvr::ADDR, SystCvr::ADDR],
            [0xE000_E010, 0xE000_E014, 0xE000_E018]
        );
        assert_eq!(SystRvr::NAME, "SYST_RVR");
        assert_eq!(size_of::<Reg<SystCsr>>(), 0);
    }
}
