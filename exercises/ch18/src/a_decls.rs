//! Задача A: объявите пять функций стандартной библиотеки C и оберните их
//! безопасно. Условие — раздел 18.7.

use std::ffi::CString;

// ANCHOR: skeleton
// Прототипы C — объявите их в блоке `unsafe extern "C"` с точными типами:
//   size_t strlen(const char *s);
//   int    abs(int x);
//   int    memcmp(const void *a, const void *b, size_t n);
//   int    toupper(int c);
//   char  *strchr(const char *s, int c);

/// Длина C-строки (возврат `size_t`).
pub fn c_strlen(s: &CString) -> usize {
    let _ = s;
    todo!("раздел 18.7, задача A")
}

/// Модуль `int`; `None` на `i32::MIN` (в C это UB).
pub fn c_abs(x: i32) -> Option<i32> {
    let _ = x;
    todo!("раздел 18.7, задача A")
}

/// `memcmp` двух срезов равной длины.
pub fn c_memcmp(a: &[u8], b: &[u8]) -> i32 {
    let _ = (a, b);
    todo!("раздел 18.7, задача A")
}

/// Заглавная буква для ASCII (аргумент `toupper` — `int`, не `char`).
pub fn c_toupper(c: u8) -> u8 {
    let _ = c;
    todo!("раздел 18.7, задача A")
}

/// Смещение первого вхождения байта или `None`.
pub fn c_strchr(s: &CString, byte: u8) -> Option<usize> {
    let _ = (s, byte);
    todo!("раздел 18.7, задача A")
}
// ANCHOR_END: skeleton
