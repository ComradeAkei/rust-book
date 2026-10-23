//! Раздел 15.2: `String` и `&str`. Строка — это `Vec<u8>` с обещанием, что
//! байты образуют UTF-8; длина считается в байтах, символ занимает от
//! одного до четырёх, поэтому номера символа по индексу нет.
//!
//! ```compile_fail,E0277
//! let s = String::from("датчик");
//! let c = s[0];
//! ```
//!
//! Срез по байтам допустим только на границе символа:
//!
//! ```should_panic
//! let s = "датчик";
//! let _ = &s[..1]; // середина «д»
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use core::str::Utf8Error;

// ANCHOR: lengths
/// Длина в байтах, в символах Unicode и размер самой ссылки `&str`.
///
/// ```
/// assert_eq!(ch15::strings::lengths("adc0"), (4, 4));
/// assert_eq!(ch15::strings::lengths("датчик"), (12, 6));
/// assert_eq!(ch15::strings::lengths("🌡"), (4, 1));
/// ```
pub fn lengths(s: &str) -> (usize, usize) {
    (s.len(), s.chars().count())
}
// ANCHOR_END: lengths

// ANCHOR: truncate
/// Префикс не длиннее `max` байт, обрезанный по границе символа: так имя
/// датчика влезает в поле фиксированной длины, как `char name[16]` в C, и
/// остаётся корректным UTF-8.
///
/// ```
/// use ch15::strings::truncate_bytes;
/// assert_eq!(truncate_bytes("датчик", 5), "да");
/// assert_eq!(truncate_bytes("adc0", 16), "adc0");
/// ```
pub fn truncate_bytes(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
// ANCHOR_END: truncate

// ANCHOR: from_wire
/// Имя датчика из байтов кадра: проверка UTF-8 один раз, на границе
/// программы. Дальше `&str` несёт это знание в типе.
///
/// ```
/// use ch15::strings::name_from_wire;
/// assert_eq!(name_from_wire(b"adc0\0\0\0\0"), Ok("adc0"));
/// let e = name_from_wire(&[0xd0, 0x64, 0]).unwrap_err();
/// assert_eq!(e.valid_up_to(), 0);
/// assert_eq!(e.to_string(), "invalid utf-8 sequence of 1 bytes from index 0");
/// ```
pub fn name_from_wire(field: &[u8]) -> Result<&str, Utf8Error> {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    core::str::from_utf8(&field[..end])
}
// ANCHOR_END: from_wire

// ANCHOR: case
/// Смена регистра меняет длину: «ß» становится «SS», и строку нельзя
/// переписать на месте, как `toupper` в цикле по `char *`.
///
/// ```
/// assert_eq!(ch15::strings::upper("straße"), ("STRASSE".to_string(), 7, 7));
/// assert_eq!(ch15::strings::upper("датчик"), ("ДАТЧИК".to_string(), 12, 12));
/// ```
pub fn upper(s: &str) -> (String, usize, usize) {
    let u = s.to_uppercase();
    let n = u.len();
    (u, s.len(), n)
}
// ANCHOR_END: case

// ANCHOR: build
/// Сборка строки без лишних выделений: ёмкость заранее и `write!` прямо в
/// `String`, без промежуточного `format!` на каждое поле.
pub fn report(names: &[&str], values: &[i32]) -> String {
    use core::fmt::Write;
    let mut out = String::with_capacity(names.len() * 16);
    for (n, v) in names.iter().zip(values) {
        // Запись в String не завершается ошибкой.
        let _ = writeln!(out, "{n}: {v}");
    }
    out
}
// ANCHOR_END: build

/// Байты, символы и их смещения: три способа обойти строку.
pub fn walks(s: &str) -> (Vec<u8>, Vec<char>, Vec<(usize, char)>) {
    (s.bytes().collect(), s.chars().collect(), s.char_indices().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_differ() {
        let (b, c, ci) = walks("дt");
        assert_eq!(b, [0xd0, 0xb4, b't']);
        assert_eq!(c, ['д', 't']);
        assert_eq!(ci, [(0, 'д'), (2, 't')]);
    }

    #[test]
    fn report_fits_capacity() {
        let r = report(&["adc0", "i2c1"], &[1250, -40]);
        assert_eq!(r, "adc0: 1250\ni2c1: -40\n");
        assert_eq!(r.capacity(), 32);
    }

    #[test]
    fn truncate_never_splits() {
        let s = "aд🌡b";
        for max in 0..=s.len() {
            let t = truncate_bytes(s, max);
            assert!(t.len() <= max && s.starts_with(t));
        }
    }
}
