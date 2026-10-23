//! Задача A: каталог объявлений FFI. Шесть функций стандартной библиотеки C;
//! для каждой дан прототип, и задача — объявить её в блоке `unsafe extern "C"`
//! с точными типами и обернуть безопасно. Частые ошибки в объявлениях — в
//! разборе (приложение E): перепутанная ширина (`int`/`long`/`size_t`),
//! знак, константность и то, что аргумент `toupper` — `int`, а не `char`.

use core::ffi::{c_char, c_int, c_void};
use std::ffi::CString;

// ANCHOR: decls
unsafe extern "C" {
    // size_t strlen(const char *s);
    fn strlen(s: *const c_char) -> usize;
    // int abs(int x);
    fn abs(x: c_int) -> c_int;
    // int memcmp(const void *a, const void *b, size_t n);
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    // int toupper(int c);
    fn toupper(c: c_int) -> c_int;
    // char *strchr(const char *s, int c);
    fn strchr(s: *const c_char, c: c_int) -> *const c_char;
}
// ANCHOR_END: decls

/// Длина C-строки. Возврат `size_t` — это `usize`, а не `i32`.
pub fn c_strlen(s: &CString) -> usize {
    // SAFETY: s — валидная C-строка с нулём на конце, живая на время вызова.
    unsafe { strlen(s.as_ptr()) }
}

/// Модуль `int`. На `i32::MIN` это UB в C (результат непредставим), поэтому
/// обёртка такой вход отвергает.
pub fn c_abs(x: i32) -> Option<i32> {
    if x == i32::MIN {
        return None;
    }
    // SAFETY: x != i32::MIN, значит |x| представимо; иных предусловий нет.
    Some(unsafe { abs(x) })
}

/// `memcmp` двух срезов равной длины: знак как у C (<0, 0, >0).
pub fn c_memcmp(a: &[u8], b: &[u8]) -> i32 {
    assert_eq!(a.len(), b.len());
    // SAFETY: обе области в границах своих срезов длины n; memcmp только читает.
    unsafe { memcmp(a.as_ptr().cast(), b.as_ptr().cast(), a.len()) }
}

/// Заглавная буква для ASCII. Аргумент `toupper` — `int` (как в C), а не
/// `char`: сужение до `c_char` сломало бы соглашение о вызовах.
pub fn c_toupper(c: u8) -> u8 {
    // SAFETY: toupper определён для значений unsigned char и EOF; u8 попадает
    // в этот диапазон.
    (unsafe { toupper(c_int::from(c)) }) as u8
}

/// Смещение первого вхождения байта или `None`. `strchr` возвращает указатель
/// внутрь той же строки — переводим его в смещение через `offset_from`.
pub fn c_strchr(s: &CString, byte: u8) -> Option<usize> {
    let base = s.as_ptr();
    // SAFETY: s — валидная C-строка; strchr вернёт указатель внутрь неё либо
    // null; при не-null оба указателя в одном выделении — offset_from законен.
    let found = unsafe { strchr(base, c_int::from(byte)) };
    if found.is_null() { None } else { Some(unsafe { found.offset_from(base) } as usize) }
}
