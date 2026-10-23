//! Раздел 11.3: какой трейт годится для `dyn`.
//!
//! Таблица методов строится один раз для пары «тип — трейт», и в ней
//! должен найтись адрес каждого метода, который можно вызвать у объекта.
//! Обобщённому методу нужна своя копия на каждый тип аргумента, и в
//! таблицу его не положить:
//!
//! ```compile_fail,E0038
//! use core::fmt;
//! pub trait Probe {
//!     fn read(&self) -> i32;
//!     fn log<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
//!         write!(w, "{}", self.read())
//!     }
//! }
//! pub fn report(p: &dyn Probe) -> i32 {
//!     p.read()
//! }
//! ```
//!
//! Метод, который возвращает `Self` по значению: размер результата
//! зависит от типа, которого вызывающий не знает.
//!
//! ```compile_fail,E0038
//! pub trait Reset {
//!     fn fresh(&self) -> Self;
//! }
//! pub fn f(_: &dyn Reset) {}
//! ```
//!
//! `Self` в параметре: второй аргумент тоже должен быть того же
//! неизвестного типа.
//!
//! ```compile_fail,E0038
//! pub trait Same {
//!     fn same(&self, other: &Self) -> bool;
//! }
//! pub fn f(_: &dyn Same) {}
//! ```
//!
//! Функция без `self`: у объекта нечем выбрать реализацию.
//!
//! ```compile_fail,E0038
//! pub trait Kind {
//!     fn kind() -> u8;
//! }
//! pub fn f(_: &dyn Kind) {}
//! ```
//!
//! Ассоциированная константа:
//!
//! ```compile_fail,E0038
//! pub trait Limits {
//!     const MAX: i32;
//! }
//! pub fn f(_: &dyn Limits) {}
//! ```
//!
//! Супертрейт, который требует `Self: Sized`, например `Clone`:
//!
//! ```compile_fail,E0038
//! pub trait Snapshot: Clone {
//!     fn read(&self) -> i32;
//! }
//! pub fn f(_: &dyn Snapshot) {}
//! ```
//!
//! `impl Trait` в результате метода (раздел 10.3): у каждой реализации свой
//! скрытый тип.
//!
//! ```compile_fail,E0038
//! pub trait Source {
//!     fn samples(&self) -> impl Iterator<Item = i16>;
//! }
//! pub fn f(_: &dyn Source) {}
//! ```
//!
//! `async fn` в трейте: тот же скрытый тип будущего, что и у `impl Future`.
//!
//! ```compile_fail,E0038
//! pub trait Feed {
//!     async fn next(&self) -> i32;
//! }
//! pub fn f(_: &dyn Feed) {}
//! ```
//!
//! Метод, который забирает `self` по значению, совместимости не мешает,
//! но у объекта его не вызвать: значение без размера не переместить.
//!
//! ```compile_fail,E0161
//! pub trait Job {
//!     fn finish(self) -> i32;
//! }
//! pub fn f(j: Box<dyn Job>) -> i32 {
//!     j.finish()
//! }
//! ```
//!
//! А `self: Box<Self>` вызывается: в таблицу едет адрес, размер известен.
//!
//! ```
//! pub trait Job {
//!     fn finish(self: Box<Self>) -> i32;
//! }
//! struct Once(i32);
//! impl Job for Once {
//!     fn finish(self: Box<Self>) -> i32 {
//!         self.0
//!     }
//! }
//! let j: Box<dyn Job> = Box::new(Once(7));
//! assert_eq!(j.finish(), 7);
//! ```
//!
//! Метод с `where Self: Sized` у объекта вызвать нельзя:
//!
//! ```compile_fail
//! use ch11::compat::Probe;
//! fn report(p: &dyn Probe, out: &mut String) -> core::fmt::Result {
//!     p.log(out)
//! }
//! ```

use core::fmt;

// ANCHOR: probe
/// Трейт, совместимый с `dyn`, в котором есть и обобщённый метод, и
/// конструктор: оба исключены из таблицы через `where Self: Sized`.
pub trait Probe {
    fn read(&self) -> i32;

    /// Есть у каждого конкретного типа, но не у `dyn Probe`.
    fn log<W: fmt::Write>(&self, w: &mut W) -> fmt::Result
    where
        Self: Sized,
    {
        write!(w, "{}", self.read())
    }

    /// Вместо параметра типа — объект: метод попадает в таблицу, и его
    /// можно вызвать через `&dyn Probe`.
    fn log_dyn(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "{}", self.read())
    }

    /// Конструктор: у объекта его не вызвать, тип неизвестен.
    fn zero() -> Self
    where
        Self: Sized;
}
// ANCHOR_END: probe

// ANCHOR: probe-impl
pub struct Adc {
    pub code: u16,
}

impl Probe for Adc {
    fn read(&self) -> i32 {
        i32::from(self.code) * 3300 / 4095
    }

    fn zero() -> Self {
        Adc { code: 0 }
    }
}

/// Через объект доступны `read` и `log_dyn`.
#[unsafe(no_mangle)]
pub fn ch11_log_dyn(p: &dyn Probe, w: &mut dyn fmt::Write) -> fmt::Result {
    p.log_dyn(w)
}

/// У конкретного типа — все методы, и `log` мономорфизуется.
pub fn log_adc(a: &Adc, w: &mut impl fmt::Write) -> fmt::Result {
    a.log(w)
}

#[unsafe(no_mangle)]
pub fn ch11_as_probe(a: &Adc) -> &dyn Probe {
    a
}
// ANCHOR_END: probe-impl

// ANCHOR: guard
// Трейт задуман для объектов: если он перестанет быть совместимым с
// `dyn`, крейт не соберётся.
const _: Option<&dyn Probe> = None;
// ANCHOR_END: guard

#[cfg(feature = "alloc")]
pub use clone_box::*;

#[cfg(feature = "alloc")]
mod clone_box {
    use crate::fat::Sensor;
    use alloc::boxed::Box;

    // ANCHOR: clone-box
    /// `Clone` несовместим с `dyn`, но копию объекта можно вернуть в
    /// куче: `Box<dyn SensorBox>` имеет известный размер.
    pub trait SensorBox: Sensor {
        fn clone_box(&self) -> Box<dyn SensorBox>;
    }

    impl<T: Sensor + Clone + 'static> SensorBox for T {
        fn clone_box(&self) -> Box<dyn SensorBox> {
            Box::new(self.clone())
        }
    }

    impl Clone for Box<dyn SensorBox> {
        fn clone(&self) -> Self {
            (**self).clone_box()
        }
    }
    // ANCHOR_END: clone-box
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fat::{Counter, Thermo};

    #[test]
    fn sized_only_methods() {
        let a = Adc { code: 4095 };
        let mut s = String::new();
        log_adc(&a, &mut s).unwrap();
        s.push(' ');
        ch11_log_dyn(ch11_as_probe(&a), &mut s).unwrap();
        assert_eq!(s, "3300 3300");
        assert_eq!(Adc::zero().read(), 0);
    }

    #[test]
    fn boxed_clones() {
        let v: Vec<Box<dyn SensorBox>> =
            vec![Box::new(Thermo { raw: 32 }), Box::new(Counter { pulses: 3 })];
        let copies = v.clone();
        let sum: i32 = copies.iter().map(|s| s.read()).sum();
        assert_eq!(sum, 203);
        assert_eq!(copies[0].name(), "термодатчик");
    }
}
