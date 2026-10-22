//! Раздел 9.6: стандартные трейты — операторы, преобразования, `AsRef` и
//! `Borrow`, `Default`, `Debug` и `Display`.
//!
//! Оператор — вызов метода трейта, и без реализации его нет:
//!
//! ```compile_fail,E0369
//! use ch09::std_traits::Q15;
//! let x = Q15::HALF / Q15::HALF;
//! ```
//!
//! Из `TryFrom` следует `TryInto`, из `From` — `Into`, писать вторые не
//! нужно:
//!
//! ```
//! use ch09::std_traits::Q15;
//! let q: Q15 = 16384i32.try_into().unwrap();
//! assert_eq!(q, Q15::HALF);
//! let raw: i32 = q.into();
//! assert_eq!(raw, 16384);
//! ```

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;
#[cfg(feature = "alloc")]
use alloc::string::String;
use core::fmt;
use core::ops::{Add, Mul, Neg, Sub};

// ANCHOR: q15
/// Число в формате Q15: `i16`, где 32767 — это почти 1.0. Арифметика с
/// насыщением, как в DSP-инструкциях Cortex-M4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Q15(pub i16);

impl Q15 {
    pub const ONE: Q15 = Q15(i16::MAX);
    pub const HALF: Q15 = Q15(1 << 14);
}

impl Add for Q15 {
    type Output = Q15;

    fn add(self, rhs: Q15) -> Q15 {
        Q15(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Q15 {
    type Output = Q15;

    fn sub(self, rhs: Q15) -> Q15 {
        Q15(self.0.saturating_sub(rhs.0))
    }
}

impl Mul for Q15 {
    type Output = Q15;

    /// Произведение двух Q15 — Q30; сдвиг на 15 с округлением возвращает
    /// Q15. Единственный случай насыщения: −1 · −1.
    fn mul(self, rhs: Q15) -> Q15 {
        let p = (i32::from(self.0) * i32::from(rhs.0) + (1 << 14)) >> 15;
        Q15(p.clamp(i16::MIN.into(), i16::MAX.into()) as i16)
    }
}

impl Neg for Q15 {
    type Output = Q15;

    fn neg(self) -> Q15 {
        Q15(self.0.saturating_neg())
    }
}
// ANCHOR_END: q15

// ANCHOR: asm
#[unsafe(no_mangle)]
pub fn ch09_q15_add(a: Q15, b: Q15) -> Q15 {
    a + b
}

#[unsafe(no_mangle)]
pub fn ch09_q15_mac(acc: Q15, a: Q15, b: Q15) -> Q15 {
    acc + a * b
}
// ANCHOR_END: asm

// ANCHOR: convert
/// Значение вне диапазона Q15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfRange(pub i32);

impl TryFrom<i32> for Q15 {
    type Error = OutOfRange;

    fn try_from(v: i32) -> Result<Q15, OutOfRange> {
        i16::try_from(v).map(Q15).map_err(|_| OutOfRange(v))
    }
}

impl From<Q15> for i32 {
    fn from(q: Q15) -> i32 {
        q.0.into()
    }
}
// ANCHOR_END: convert

// ANCHOR: display
/// `Debug` выведен и показывает устройство (`Q15(16384)`), `Display`
/// написан руками и показывает смысл (`0.5000`).
impl fmt::Display for Q15 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Четыре знака после запятой, округление к ближайшему.
        let v = i32::from(self.0);
        let sign = if v < 0 { "-" } else { "" };
        let scaled = (v.unsigned_abs() * 10_000 + (1 << 14)) >> 15;
        write!(f, "{sign}{}.{:04}", scaled / 10_000, scaled % 10_000)
    }
}
// ANCHOR_END: display

// ANCHOR: asref
/// CRC-8 (полином 0x07) над чем угодно, что можно увидеть как байты:
/// `&[u8]`, `[u8; N]`, `Vec<u8>`, `&str`, `String`.
pub fn crc8<T: AsRef<[u8]>>(data: T) -> u8 {
    data.as_ref().iter().fold(0, |crc, &b| {
        let mut c = crc ^ b;
        for _ in 0..8 {
            c = if c & 0x80 != 0 { (c << 1) ^ 0x07 } else { c << 1 };
        }
        c
    })
}
// ANCHOR_END: asref

// ANCHOR: borrow
/// Ключи — `String`, а искать можно по `&str`: `String: Borrow<str>`, и
/// сравнение у `str` то же, что у `String`.
#[cfg(feature = "alloc")]
pub fn calibration() -> BTreeMap<String, Q15> {
    let mut m = BTreeMap::new();
    m.insert(String::from("thermo"), Q15::HALF);
    m.insert(String::from("baro"), Q15::ONE);
    m
}

#[cfg(feature = "alloc")]
pub fn gain(map: &BTreeMap<String, Q15>, sensor: &str) -> Option<Q15> {
    map.get(sensor).copied()
}
// ANCHOR_END: borrow

// ANCHOR: default
/// Настройки опроса. `Default` выведен для всей структуры, потому что он
/// есть у каждого поля; у `period_ms` разумный ноль быть не может, поэтому
/// реализация ручная.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollConfig {
    pub period_ms: u32,
    pub gain: Q15,
    pub retries: u8,
}

impl Default for PollConfig {
    fn default() -> Self {
        PollConfig { period_ms: 100, gain: Q15::ONE, retries: 3 }
    }
}
// ANCHOR_END: default

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn q15_arithmetic() {
        assert_eq!(Q15::HALF + Q15::HALF, Q15(32767)); // насыщение, а не -32768
        assert_eq!(Q15(-32768) - Q15(1), Q15(-32768));
        assert_eq!(Q15::HALF * Q15::HALF, Q15(8192));
        assert_eq!(Q15(-32768) * Q15(-32768), Q15::ONE);
        assert_eq!(-Q15(-32768), Q15::ONE);
        assert_eq!(ch09_q15_mac(Q15(100), Q15::HALF, Q15(-200)), Q15(0));
        assert_eq!(ch09_q15_add(Q15(1), Q15(2)), Q15(3));
    }

    #[test]
    fn conversions() {
        assert_eq!(Q15::try_from(40_000), Err(OutOfRange(40_000)));
        assert_eq!(i32::from(Q15(-5)), -5);
    }

    #[test]
    fn formatting() {
        assert_eq!(format!("{:?}", Q15::HALF), "Q15(16384)");
        assert_eq!(Q15::HALF.to_string(), "0.5000");
        assert_eq!(Q15(-32768).to_string(), "-1.0000");
        assert_eq!(Q15(1).to_string(), "0.0000");
        assert_eq!(Q15::ONE.to_string(), "1.0000");
        assert_eq!(Q15(-8192).to_string(), "-0.2500");
    }

    #[test]
    fn as_ref_and_borrow() {
        let s = String::from("123456789");
        assert_eq!(crc8("123456789"), 0xf4);
        assert_eq!(crc8(&s), 0xf4);
        assert_eq!(crc8(s.clone()), 0xf4);
        assert_eq!(crc8(*b"123456789"), 0xf4);
        assert_eq!(crc8(vec![0u8; 0]), 0);
        let m = calibration();
        assert_eq!(gain(&m, "thermo"), Some(Q15::HALF));
        assert_eq!(gain(&m, "hygro"), None);
    }

    #[test]
    fn defaults() {
        let c = PollConfig { retries: 5, ..Default::default() };
        assert_eq!(c, PollConfig { period_ms: 100, gain: Q15::ONE, retries: 5 });
        assert_eq!(Q15::default(), Q15(0));
    }
}
