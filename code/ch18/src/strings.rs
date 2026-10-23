//! Раздел 18.3: строки и `core::ffi`. Строка в C — это указатель на байты,
//! оканчивающиеся нулём, без длины и без обещания про UTF-8. Строка в Rust —
//! `&str`: длина плюс гарантия UTF-8, нуль внутри разрешён. Мост между ними —
//! `CString` (владеющая, с нулём на конце) и `CStr` (заём до нуля).

use crate::layout::Kind;
use core::ffi::{c_char, c_int};
use std::ffi::{CStr, CString};

unsafe extern "C" {
    fn c_name_len(s: *const c_char) -> usize;
    fn c_unit_name(kind: c_int) -> *const c_char;
}

// ANCHOR: into_c
/// Передать Rust-строку в C. `CString::new` дописывает завершающий нуль и
/// проверяет, что внутри строки нуля нет, — иначе C увидел бы её обрезанной,
/// и вернётся `None`.
pub fn name_len(name: &str) -> Option<usize> {
    let c = CString::new(name).ok()?; // Err, если внутри есть \0
    // SAFETY: c.as_ptr() указывает на строку с завершающим нулём, живую до
    // конца выражения (c жива до точки с запятой); c_name_len только читает.
    Some(unsafe { c_name_len(c.as_ptr()) })
}
// ANCHOR_END: into_c

// ANCHOR: cliteral
/// Литерал `c"..."` — готовая C-строка с нулём на конце, построенная во время
/// компиляции, без выделения кучи (в отличие от `CString`).
pub const UNKNOWN: &CStr = c"n/a";
// ANCHOR_END: cliteral

// ANCHOR: from_c
/// Прочитать статическую строку, которую вернул C. `CStr::from_ptr` — заём до
/// первого нуля; байты могут быть не UTF-8 (единица «температура» в нашем C —
/// Latin-1), поэтому `to_str()` возвращает `Result`, а не паникует.
pub fn unit_name(kind: Kind) -> Result<&'static str, &'static CStr> {
    // SAFETY: c_unit_name возвращает указатель на статическую строку с нулём,
    // живущую всю программу, — значит заём 'static корректен.
    let cstr = unsafe { CStr::from_ptr(c_unit_name(kind as c_int)) };
    cstr.to_str().map_err(|_| cstr)
}
// ANCHOR_END: from_c

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "вызывает C")]
    fn name_len_matches_and_rejects_interior_nul() {
        assert_eq!(name_len("температура"), Some("температура".len()));
        assert_eq!(name_len("a\0b"), None); // внутренний нуль
    }

    #[test]
    #[cfg_attr(miri, ignore = "вызывает C")]
    fn c_literal_has_trailing_nul() {
        // SAFETY: UNKNOWN — валидная C-строка с нулём.
        assert_eq!(unsafe { c_name_len(UNKNOWN.as_ptr()) }, 3);
    }

    #[test]
    #[cfg_attr(miri, ignore = "вызывает C")]
    fn unit_name_handles_non_utf8() {
        assert_eq!(unit_name(Kind::Voltage), Ok("mV"));
        // «температура» — Latin-1, не UTF-8: to_str() должен вернуть Err,
        // а lossy-преобразование — заменить сбойный байт.
        let temp = unit_name(Kind::Temperature);
        assert!(temp.is_err());
        let cstr = temp.unwrap_err();
        assert!(cstr.to_string_lossy().ends_with('C'));
    }
}
