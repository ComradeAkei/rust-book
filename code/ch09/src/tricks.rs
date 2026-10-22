//! Раздел 9.7: запечатанный трейт, трейт-расширение и ловушка `Deref`.
//!
//! Doc-тест — другой крейт. Реализовать запечатанный трейт ему нельзя:
//! супертрейт `Sealed` лежит в приватном модуле.
//!
//! ```compile_fail,E0277
//! use ch09::tricks::Peripheral;
//! struct MyUart;
//! impl Peripheral for MyUart {
//!     const BASE: usize = 0x4000_D000;
//! }
//! ```
//!
//! ```compile_fail,E0603
//! struct MyUart;
//! impl ch09::tricks::private::Sealed for MyUart {}
//! ```
//!
//! А пользоваться — можно:
//!
//! ```
//! use ch09::tricks::{Peripheral, Uart0};
//! fn base<P: Peripheral>() -> usize { P::BASE }
//! assert_eq!(base::<Uart0>(), 0x4000_C000);
//! ```
//!
//! Метод трейта-расширения виден, только когда трейт импортирован:
//!
//! ```compile_fail,E0599
//! let frame = [1u8, 2, 3];
//! let _ = frame.checksum();
//! ```
//!
//! ```
//! use ch09::tricks::SliceExt;
//! let frame = [1u8, 2, 3];
//! assert_eq!(frame.checksum(), 0xfa);
//! ```
//!
//! `Deref` делает обёртку похожей на то, что внутри, вплоть до `clone`:
//! у `Uart` нет `Clone`, и метод находится у `Regs`.
//!
//! ```compile_fail,E0308
//! use ch09::tricks::{Regs, Uart};
//! let u = Uart::new(Regs { base: 0x4000_C000 });
//! let copy: Uart = u.clone();
//! ```

use core::ops::Deref;

// ANCHOR: sealed
mod private {
    /// Печать: модуль `private` не виден снаружи крейта, поэтому назвать
    /// `Sealed`, а значит, и реализовать его, другой крейт не может.
    pub trait Sealed {}
}

/// Блок периферии микроконтроллера. Список закрыт: только UART этого
/// крейта, и добавить свой пользователь не может. Значит, крейт волен
/// добавлять в трейт методы без значения по умолчанию, не ломая чужой код.
pub trait Peripheral: private::Sealed {
    const BASE: usize;
}

pub struct Uart0;
pub struct Uart1;

impl private::Sealed for Uart0 {}
impl private::Sealed for Uart1 {}

impl Peripheral for Uart0 {
    const BASE: usize = 0x4000_C000;
}

impl Peripheral for Uart1 {
    const BASE: usize = 0x4000_D000;
}
// ANCHOR_END: sealed

// ANCHOR: extension
/// Трейт-расширение: новый метод для чужого типа `[u8]`. Чужой тип нельзя
/// дополнить inherent-методом, а свой трейт для него реализовать можно.
pub trait SliceExt {
    /// Дополнение до нуля суммы байтов по модулю 256, как в Intel HEX.
    fn checksum(&self) -> u8;
}

impl SliceExt for [u8] {
    fn checksum(&self) -> u8 {
        self.iter().fold(0u8, |s, &b| s.wrapping_add(b)).wrapping_neg()
    }
}
// ANCHOR_END: extension

// ANCHOR: deref
/// Регистры UART: `Clone` выведен, копия описания ничего не стоит.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regs {
    pub base: usize,
}

impl Regs {
    pub fn enable(&self) -> &'static str {
        "Regs::enable: только бит UARTEN"
    }
}

/// Драйвер «наследует» регистры через `Deref`. Так делать не надо.
pub struct Uart {
    regs: Regs,
    pub baud: u32,
}

impl Uart {
    pub fn new(regs: Regs) -> Self {
        Uart { regs, baud: 0 }
    }
}

impl Deref for Uart {
    type Target = Regs;

    fn deref(&self) -> &Regs {
        &self.regs
    }
}
// ANCHOR_END: deref

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_and_extension() {
        assert_eq!(Uart1::BASE - Uart0::BASE, 0x1000);
        // :10 0100 00 214601360121470136007EFE09D21901 40 — строка Intel HEX.
        let rec = [
            0x10, 0x01, 0x00, 0x00, 0x21, 0x46, 0x01, 0x36, 0x01, 0x21, 0x47, 0x01, 0x36, 0x00,
            0x7e, 0xfe, 0x09, 0xd2, 0x19, 0x01,
        ];
        assert_eq!(rec.checksum(), 0x40);
    }

    #[test]
    fn deref_lookalike() {
        let u = Uart::new(Regs { base: 0x4000_C000 });
        // Метод и clone найдены у Regs: Uart их не объявлял.
        assert_eq!(u.enable(), "Regs::enable: только бит UARTEN");
        let c = u.clone();
        assert_eq!(c, Regs { base: 0x4000_C000 });
        assert_eq!(u.baud, 0);
    }
}
