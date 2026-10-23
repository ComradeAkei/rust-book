//! Раздел 26.1: один и тот же отчёт о показании датчика на трёх слоях
//! стандартной библиотеки.
//!
//! - `core`: форматирование есть, памяти для результата нет — пишем в
//!   буфер вызывающего ([`report_into`]).
//! - `alloc`: есть куча, результат — `String` ([`report`]).
//! - `std`: есть ОС, отчёт можно напечатать ([`print_report`]).

use core::fmt::{self, Write};

/// Показание: канал и значение в сотых долях единицы.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    pub channel: u8,
    pub centi: i32,
}

impl fmt::Display for Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.centi < 0 { "-" } else { "" };
        let v = self.centi.unsigned_abs();
        write!(f, "канал {}: {sign}{}.{:02}", self.channel, v / 100, v % 100)
    }
}

// ANCHOR: cursor
/// Запись в срез байт через `core::fmt::Write`: место кончилось — ошибка,
/// а не выделение памяти.
pub struct Cursor<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Cursor { buf, len: 0 }
    }

    /// Записанное, с временем жизни буфера. Пишет сюда только
    /// `write_str`, целыми строками, поэтому байты — всегда UTF-8.
    pub fn into_str(self) -> &'a str {
        let buf: &'a [u8] = self.buf;
        core::str::from_utf8(&buf[..self.len]).unwrap()
    }
}

impl Write for Cursor<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(fmt::Error)?;
        let dst = self.buf.get_mut(self.len..end).ok_or(fmt::Error)?;
        dst.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// Слой `core`: отчёт в буфер вызывающего.
pub fn report_into<'a>(buf: &'a mut [u8], r: &Reading) -> Result<&'a str, fmt::Error> {
    let mut c = Cursor::new(buf);
    write!(c, "{r}")?;
    Ok(c.into_str())
}
// ANCHOR_END: cursor

// ANCHOR: alloc
/// Слой `alloc`: отчёт в `String` — нужна куча, но не ОС.
#[cfg(feature = "alloc")]
pub fn report(r: &Reading) -> alloc::string::String {
    alloc::format!("{r}")
}

/// Слой `std`: печать в stdout — нужна ОС.
#[cfg(feature = "std")]
pub fn print_report(r: &Reading) -> std::io::Result<()> {
    use std::io::Write as _;
    writeln!(std::io::stdout().lock(), "{r}")
}
// ANCHOR_END: alloc

#[cfg(test)]
mod tests {
    // ANCHOR: tests
    // Крейт — no_std, а тесты собираются с std: её подключает тестовая
    // обвязка. Но в область видимости std не попадает, макросы vec! и
    // format! без этих строк не найдутся.
    extern crate std;
    use std::{format, string::ToString};
    // ANCHOR_END: tests

    use super::*;

    #[test]
    fn three_layers_agree() {
        let r = Reading { channel: 3, centi: -1205 };
        let mut buf = [0u8; 32];
        let s = report_into(&mut buf, &r).unwrap();
        assert_eq!(s, "канал 3: -12.05");
        assert_eq!(report(&r), s);
        assert_eq!(format!("{r}"), s);
        assert_eq!(r.to_string(), s);
    }

    #[test]
    fn small_buffer_is_an_error() {
        let r = Reading { channel: 1, centi: 7 };
        let mut buf = [0u8; 8];
        assert!(report_into(&mut buf, &r).is_err());
        let mut exact = [0u8; 18];
        assert_eq!(report_into(&mut exact, &r).unwrap(), "канал 1: 0.07");
    }
}
