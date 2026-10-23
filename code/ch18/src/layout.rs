//! Раздел 18.2: раскладка на границе. Чтобы передать структуру в C, её
//! раскладка в памяти должна совпадать с C-структурой. Обычный Rust-тип не
//! годится: компилятор вправе переставить поля и выбрать своё выравнивание.
//! `#[repr(C)]` фиксирует порядок и выравнивание как в C; `#[repr(transparent)]`
//! делает тип неотличимым от единственного поля.

use core::ffi::c_int;

// ANCHOR: repr
/// Та же раскладка, что у `struct CPoint` в C: два `int` подряд.
/// `#[repr(C)]` запрещает перестановку полей и даёт C-выравнивание, поэтому
/// структуру можно передавать по значению в обе стороны.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: c_int,
    pub y: c_int,
}

unsafe extern "C" {
    fn c_point_sum(p: Point) -> c_int;
    fn c_point_new(x: c_int, y: c_int) -> Point;
}

/// Передать структуру в C по значению.
pub fn sum(p: Point) -> i32 {
    // SAFETY: Point имеет раскладку struct CPoint (#[repr(C)]), передача по
    // значению следует тому же C-соглашению; функция только читает поля.
    unsafe { c_point_sum(p) }
}

/// Получить структуру из C по значению.
pub fn make(x: i32, y: i32) -> Point {
    // SAFETY: см. `sum`; C возвращает корректный CPoint той же раскладки.
    unsafe { c_point_new(x, y) }
}
// ANCHOR_END: repr

// ANCHOR: enum
/// Перечисление с C-представлением: в памяти это `int` с заданными
/// значениями. Без `#[repr(C)]` (или `#[repr(i32)]`) размер и значения
/// дискриминанта — на усмотрение компилятора, и в C их передавать нельзя.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Voltage = 0,
    Temperature = 1,
    Pressure = 2,
}
// ANCHOR_END: enum

// ANCHOR: transparent
/// `#[repr(transparent)]`: в памяти неотличим от `c_int`, поэтому передаётся
/// в C всюду, где ждут `int`, но в Rust остаётся отдельным типом — единицы
/// не перепутать. Так устроены `NonZero*` и обёртки стандартной библиотеки.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Millivolts(pub c_int);
// ANCHOR_END: transparent

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "вызывает C")]
    fn struct_by_value_both_ways() {
        let p = make(3, 4);
        assert_eq!(p, Point { x: 3, y: 4 });
        assert_eq!(sum(p), 7);
    }

    #[test]
    fn transparent_is_size_of_inner() {
        assert_eq!(size_of::<Millivolts>(), size_of::<c_int>());
        assert_eq!(Kind::Pressure as c_int, 2);
    }
}
