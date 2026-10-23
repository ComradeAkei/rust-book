//! Задача A: каталог из восьми вызовов `transmute`. Три корректны (обычные
//! тесты ниже), три дают неопределённое поведение (помечены `#[ignore]` и
//! ловятся Miri поодиночке — `code/ch17/ub/run.sh`), два не компилируются
//! (doc-тесты `compile_fail` в `lib.rs`). Разбор — в приложении E.

// Весь файл — каталог transmute. Компилятор и clippy справедливо
// предлагают безопасные замены (это и есть урок задачи A), но здесь
// показан именно transmute.
#![allow(unnecessary_transmutes)]
#![allow(clippy::transmute_int_to_bool, clippy::transmute_int_to_non_zero)]

use std::hint::black_box;

// ----- Корректные: transmute законен, но у каждого есть безопасная замена -----

#[test]
fn a_ok_bytes_to_u32() {
    // [u8; 4] и u32 одной раскладки, любые биты валидны для u32.
    let v: u32 = unsafe { core::mem::transmute([0x78u8, 0x56, 0x34, 0x12]) };
    assert_eq!(v, u32::from_le_bytes([0x78, 0x56, 0x34, 0x12]));
}

#[test]
fn a_ok_u32_to_bytes() {
    let b: [u8; 4] = unsafe { core::mem::transmute(0x1234_5678u32) };
    assert_eq!(u32::from_ne_bytes(b), 0x1234_5678);
}

// ANCHOR: a_ok
#[test]
fn a_ok_f32_bits() {
    // f32 → u32: размеры равны, любой образец валиден для u32.
    let bits: u32 = unsafe { core::mem::transmute(1.0f32) };
    assert_eq!(bits, 1.0f32.to_bits());
}
// ANCHOR_END: a_ok

// ----- UB: запускать под Miri поодиночке -----

// ANCHOR: a_ub
#[test]
#[ignore = "UB: invalid value (bool); под Miri поодиночке"]
#[allow(invalid_value)]
fn a_ub_invalid_bool() {
    // 2 не является корректным bool — UB уже при создании значения.
    let b: bool = unsafe { core::mem::transmute(2u8) };
    black_box(b);
}
// ANCHOR_END: a_ub

#[test]
#[ignore = "UB: invalid value (ниша NonZero); под Miri поодиночке"]
#[allow(invalid_value)]
fn a_ub_zero_nonzero() {
    // 0 нарушает нишу NonZeroU8 — значение недопустимо.
    let n: core::num::NonZeroU8 = unsafe { core::mem::transmute(0u8) };
    black_box(n);
}

#[test]
#[ignore = "UB: invalid value (диапазон char); под Miri поодиночке"]
#[allow(invalid_value)]
fn a_ub_bad_char() {
    // 0x11_FFFF больше максимума Unicode 0x10_FFFF — не корректный char.
    let c: char = unsafe { core::mem::transmute(0x11_FFFFu32) };
    black_box(c);
}
