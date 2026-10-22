//! Раздел 7.2: `?` вызывает `From::from`, `Box<dyn Error>`, `Infallible`
//! и `#[must_use]`.

use crate::bus::{Bus, BusError, read_temp};
use alloc::boxed::Box;
use core::convert::Infallible;
use core::error::Error;

// ANCHOR: sensor_error
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensorError {
    Bus(BusError),
    /// Значение вне диапазона датчика, −40,0…+125,0 °C.
    OutOfRange(i16),
}

impl From<BusError> for SensorError {
    fn from(e: BusError) -> Self {
        SensorError::Bus(e)
    }
}

pub fn read_checked(bus: &mut Bus) -> Result<i16, SensorError> {
    let t = read_temp(bus)?; // BusError → SensorError через From
    if !(-400..=1250).contains(&t) {
        return Err(SensorError::OutOfRange(t));
    }
    Ok(t)
}
// ANCHOR_END: sensor_error

// ANCHOR: boxed
/// Ошибка любого типа, который реализует `Error`. Удобно в прототипах и
/// в `main`, но вызывающий теряет возможность перебрать варианты.
pub fn parse_port(s: &str) -> Result<u16, Box<dyn Error + Send + Sync>> {
    let p: u16 = s.trim().parse()?; // ParseIntError → Box<dyn Error>
    if p == 0 {
        return Err("порт 0 зарезервирован".into()); // &str → Box<dyn Error>
    }
    Ok(p)
}
// ANCHOR_END: boxed

// ANCHOR: infallible
/// `u16::try_from(u8)` не может не получиться: тип ошибки — `Infallible`,
/// у которого нет значений. Поэтому образец `Ok(w)` неопровержим. Clippy
/// справедливо советует `u16::from(x)`: пример показывает сам `Infallible`.
#[allow(clippy::unnecessary_fallible_conversions)]
pub fn widen(x: u8) -> u16 {
    let Ok(w) = u16::try_from(x);
    w
}

/// Пустой enum в роли ошибки ничего не стоит.
pub const INFALLIBLE_RESULT: usize = size_of::<Result<u32, Infallible>>();
// ANCHOR_END: infallible

// ANCHOR: must_use
/// Флаг готовности результата. Без проверки флага отсчёт может оказаться
/// прошлым, поэтому значение нельзя молча выбросить.
#[must_use = "отсчёт без флага готовности может быть устаревшим"]
pub fn conversion_done(status: u32) -> bool {
    status & (1 << 7) != 0
}
// ANCHOR_END: must_use

/// Выброшенный `Result` — предупреждение `unused_must_use`. Здесь оно
/// превращено в ошибку, как в проектах с `-D warnings`:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// fn send(_: &[u8]) -> Result<usize, ()> { Ok(0) }
/// send(b"ping");
/// ```
///
/// То же для функции со своим `#[must_use]`:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// ch07::convert::conversion_done(0x80);
/// ```
///
/// `let _ =` — явный отказ от значения, он компилируется:
///
/// ```
/// #![deny(unused_must_use)]
/// fn send(_: &[u8]) -> Result<usize, ()> { Ok(0) }
/// let _ = send(b"ping");
/// ```
pub fn must_use_examples() {}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn from_on_question_mark() {
        assert_eq!(read_checked(&mut Bus::new([0x00, 0xE7, 0, 0])), Ok(231));
        assert_eq!(
            read_checked(&mut Bus::failing_at([0; 4], 0)),
            Err(SensorError::Bus(BusError::Nack(0)))
        );
        assert_eq!(
            read_checked(&mut Bus::new([0x7F, 0xFF, 0, 0])),
            Err(SensorError::OutOfRange(i16::MAX))
        );
    }

    #[test]
    fn boxed_errors() {
        assert_eq!(parse_port(" 8080 ").unwrap(), 8080);
        let e = parse_port("80x").unwrap_err();
        assert_eq!(e.to_string(), "invalid digit found in string");
        assert!(e.downcast_ref::<core::num::ParseIntError>().is_some());
        let e = parse_port("0").unwrap_err();
        assert_eq!(e.to_string(), "порт 0 зарезервирован");
        assert!(e.downcast_ref::<core::num::ParseIntError>().is_none());
    }

    #[test]
    fn infallible_is_free() {
        assert_eq!(widen(200), 200);
        assert_eq!(INFALLIBLE_RESULT, size_of::<u32>());
        assert!(conversion_done(0x80));
    }
}
