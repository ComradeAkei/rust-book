//! Задача D: размер и выравнивание из таблицы методов.
//!
//! Функции `raw_*` читают таблицу методов напрямую, по раскладке, которую
//! rustc 1.97 использует, но не обещает. Функции `std_*` должны дать то же
//! через стабильные API стандартной библиотеки.

use alloc::string::String;
use core::alloc::Layout;

// ANCHOR: types
pub trait Sensor {
    fn read(&self) -> i32;
}

/// Тип нулевого размера.
pub struct Unit;
pub struct Byte(pub u8);
pub struct Wide(pub u64);
/// Строка кэша: выравнивание 64.
#[repr(align(64))]
pub struct Line(pub [u8; 64]);
/// С деструктором: `String` освобождает буфер.
pub struct Named(pub String);

impl Sensor for Unit {
    fn read(&self) -> i32 {
        0
    }
}

impl Sensor for Byte {
    fn read(&self) -> i32 {
        i32::from(self.0)
    }
}

impl Sensor for Wide {
    fn read(&self) -> i32 {
        self.0 as i32
    }
}

impl Sensor for Line {
    fn read(&self) -> i32 {
        self.0.iter().map(|&b| i32::from(b)).sum()
    }
}

impl Sensor for Named {
    fn read(&self) -> i32 {
        self.0.len() as i32
    }
}
// ANCHOR_END: types

// ANCHOR: raw
/// Адрес данных и адрес таблицы.
pub fn split(s: &dyn Sensor) -> (*const (), *const usize) {
    todo!("разобрать толстый указатель на два слова")
}

/// Слова 1 и 2 таблицы: размер и выравнивание.
pub fn raw_size_align(s: &dyn Sensor) -> (usize, usize) {
    todo!()
}

/// Слово 0 таблицы: `drop_in_place` или ноль, если деструктора нет.
pub fn raw_has_drop(s: &dyn Sensor) -> bool {
    todo!()
}
// ANCHOR_END: raw

// ANCHOR: std
/// То же через стабильные функции стандартной библиотеки.
pub fn std_size_align(s: &dyn Sensor) -> (usize, usize) {
    todo!()
}

pub fn std_layout(s: &dyn Sensor) -> Layout {
    todo!()
}
// ANCHOR_END: std
