//! Раздел 9.4: правило сироты и когерентность.
//!
//! Doc-тест — отдельный крейт, поэтому всё, что объявлено в `ch09`, для
//! него чужое, как и `core`. Чужой трейт для чужого типа — E0117:
//!
//! ```compile_fail,E0117
//! impl core::fmt::Display for ch09::coherence::Readings {
//!     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//!         Ok(())
//!     }
//! }
//! ```
//!
//! Обобщённая реализация чужого трейта для любого типа — E0210:
//!
//! ```compile_fail,E0210
//! use ch09::sensor::Sensor;
//! impl<T: Sensor> core::fmt::Display for T {
//!     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//!         Ok(())
//!     }
//! }
//! ```
//!
//! Две реализации, которые могут подойти одному типу, — E0119, даже если
//! обе в одном крейте:
//!
//! ```compile_fail,E0119
//! use ch09::sensor::{Sensor, Thermo};
//! trait Describe { fn describe(&self) -> &'static str; }
//! impl<T: Sensor> Describe for T { fn describe(&self) -> &'static str { "датчик" } }
//! impl Describe for Thermo { fn describe(&self) -> &'static str { "термодатчик" } }
//! ```
//!
//! Компилятор не верит, что чужой тип никогда не получит чужой трейт:
//! владелец типа может добавить реализацию в следующей версии.
//!
//! ```compile_fail,E0119
//! trait Describe { fn describe(&self) -> &'static str; }
//! impl<T: core::fmt::Display> Describe for T { fn describe(&self) -> &'static str { "текст" } }
//! impl Describe for Option<u8> { fn describe(&self) -> &'static str { "может быть" } }
//! ```
//!
//! А про свой тип знает точно: реализацию для него может добавить только
//! этот же крейт.
//!
//! ```
//! trait Describe { fn describe(&self) -> &'static str; }
//! impl<T: core::fmt::Display> Describe for T { fn describe(&self) -> &'static str { "текст" } }
//! struct Raw(u8); // Display у Raw нет и не будет без ведома этого крейта
//! impl Describe for Raw { fn describe(&self) -> &'static str { "сырой отсчёт" } }
//! assert_eq!(Raw(1).describe(), "сырой отсчёт");
//! assert_eq!(5.describe(), "текст");
//! ```

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use core::fmt;

// ANCHOR: newtype
/// Обёртка над чужим `Vec<i32>`: сам `Vec` и `Display` чужие, а
/// `Readings` своя, и для неё можно реализовать что угодно.
#[cfg(feature = "alloc")]
pub struct Readings(pub Vec<i32>);

#[cfg(feature = "alloc")]
impl fmt::Display for Readings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, r) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            let sign = if *r < 0 { "-" } else { "" };
            let v = r.unsigned_abs();
            write!(f, "{sign}{}.{:02}", v / 100, v % 100)?;
        }
        Ok(())
    }
}
// ANCHOR_END: newtype

// ANCHOR: local-param
/// Чужой трейт с локальным типом в параметре: `From<Celsius>` — свой
/// трейт в смысле правила сироты, хотя `From` и `i32` чужие.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Celsius(pub i32);

impl From<Celsius> for i32 {
    fn from(c: Celsius) -> i32 {
        c.0
    }
}
// ANCHOR_END: local-param

// ANCHOR: blanket
/// Своя обобщённая реализация: всякий `Sensor` получает `Describe`.
pub trait Describe {
    fn describe(&mut self) -> Celsius;
}

impl<T: crate::sensor::Sensor> Describe for T {
    fn describe(&mut self) -> Celsius {
        Celsius(self.read())
    }
}
// ANCHOR_END: blanket

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::Thermo;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn newtype_and_conversions() {
        assert_eq!(Readings(vec![2150, -405, -5]).to_string(), "21.50, -4.05, -0.05");
        let raw: i32 = Celsius(2150).into();
        assert_eq!(raw, 2150);
        assert_eq!(Thermo { raw: 99 }.describe(), Celsius(99));
    }
}
