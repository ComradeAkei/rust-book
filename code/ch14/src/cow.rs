//! Раздел 14.7: `Cow` — заимствовать, пока можно, и владеть, когда нужно.
//!
//! Поле CSV, которое копируется только при наличии спецсимволов; нагрузка
//! кадра SLIP, которая остаётся подсрезом входа, если в ней нет
//! экранирования; `to_mut`, который копирует при первой записи.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

// ANCHOR: csv
/// Имя канала как поле CSV. Кавычки, запятые и переводы строк надо
/// экранировать, но в обычном имени их нет — и тогда копировать нечего.
pub fn csv_field(name: &str) -> Cow<'_, str> {
    if !name.contains([',', '"', '\n']) {
        return Cow::Borrowed(name);
    }
    let mut out = String::with_capacity(name.len() + 2);
    out.push('"');
    for ch in name.chars() {
        if ch == '"' {
            out.push('"');
        }
        out.push(ch);
    }
    out.push('"');
    Cow::Owned(out)
}
// ANCHOR_END: csv

// ANCHOR: unslip
pub const ESC: u8 = 0xDB;
pub const ESC_END: u8 = 0xDC;
pub const ESC_ESC: u8 = 0xDD;
pub const END: u8 = 0xC0;

/// Нагрузка кадра SLIP без экранирования. Если ESC в кадре нет — а это
/// почти всегда, — результат указывает в сам кадр. Ошибка — смещение
/// неверной пары.
pub fn unslip(frame: &[u8]) -> Result<Cow<'_, [u8]>, usize> {
    let Some(first) = frame.iter().position(|&b| b == ESC) else {
        return Ok(Cow::Borrowed(frame));
    };
    let mut out = Vec::with_capacity(frame.len());
    out.extend_from_slice(&frame[..first]);
    let mut i = first;
    while i < frame.len() {
        match frame[i] {
            ESC => {
                out.push(match frame.get(i + 1) {
                    Some(&ESC_END) => END,
                    Some(&ESC_ESC) => ESC,
                    _ => return Err(i),
                });
                i += 2;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    Ok(Cow::Owned(out))
}
// ANCHOR_END: unslip

// ANCHOR: to_mut
/// Имена каналов сравниваются без учёта регистра. `to_mut` копирует
/// строку только перед первой правкой.
pub fn lowercase(name: &str) -> Cow<'_, str> {
    let mut s = Cow::Borrowed(name);
    if name.bytes().any(|b| b.is_ascii_uppercase()) {
        s.to_mut().make_ascii_lowercase();
    }
    s
}
// ANCHOR_END: to_mut

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! borrowed {
        ($c:expr) => {
            matches!($c, Cow::Borrowed(_))
        };
    }

    #[test]
    fn csv_copies_only_special_names() {
        let a = csv_field("adc0");
        assert!(borrowed!(a));
        let b = csv_field("давление, кПа");
        assert_eq!(b, "\"давление, кПа\"");
        assert_eq!(csv_field("ось \"X\""), "\"ось \"\"X\"\"\"");
        assert!(!borrowed!(b));
    }

    #[test]
    fn unslip_borrows_without_escapes() {
        let f = [1u8, 2, 3];
        let r = unslip(&f).unwrap();
        assert!(borrowed!(r));
        assert_eq!(r.as_ptr(), f.as_ptr());
        let g = [1u8, ESC, ESC_END, 2, ESC, ESC_ESC];
        assert_eq!(&*unslip(&g).unwrap(), &[1, END, 2, ESC]);
        assert_eq!(unslip(&[1, ESC, 7]), Err(1));
        assert_eq!(unslip(&[1, ESC]), Err(1));
    }

    #[test]
    fn to_mut_copies_on_first_write() {
        assert!(borrowed!(lowercase("adc0")));
        let s = lowercase("ADC0");
        assert!(!borrowed!(s));
        assert_eq!(s, "adc0");
    }

    // Наблюдение rustc 1.97: тег Cow прячется в ёмкости String, которая не
    // бывает больше isize::MAX, и Cow<str> не больше String.
    #[test]
    fn observed_cow_size() {
        use core::mem::size_of;
        assert_eq!(size_of::<Cow<'_, str>>(), size_of::<String>());
        assert_eq!(size_of::<Option<Cow<'_, str>>>(), size_of::<String>());
        assert_eq!(size_of::<Cow<'_, [u8]>>(), size_of::<Vec<u8>>());
    }
}
