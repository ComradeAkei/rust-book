//! Задача D: размер и выравнивание из таблицы методов.
//!
//! Эталонное решение. Чтение таблицы опирается на раскладку, которую
//! rustc 1.97 использует, но не обещает; функции `std_*` дают то же через
//! стабильные API, и только они проходят Miri.

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

// ANCHOR: split-type
/// Толстый указатель как два слова. `repr(C)` фиксирует порядок полей
/// структуры; порядок слов в самом `&dyn` — наблюдение.
#[repr(C)]
struct Raw {
    data: *const (),
    vtable: *const usize,
}
// ANCHOR_END: split-type

// ANCHOR: raw
/// Адрес данных и адрес таблицы.
pub fn split(s: &dyn Sensor) -> (*const (), *const usize) {
    // SAFETY: &dyn Sensor и Raw — два указателя одного размера; что первое
    // слово — данные, а второе — таблица, rustc не обещает.
    let raw: Raw = unsafe { core::mem::transmute::<&dyn Sensor, Raw>(s) };
    (raw.data, raw.vtable)
}

/// Слова 1 и 2 таблицы: размер и выравнивание.
pub fn raw_size_align(s: &dyn Sensor) -> (usize, usize) {
    let (_, vt) = split(s);
    // SAFETY: в rustc 1.97 таблица начинается словами drop, size, align.
    // Miri таблицу памятью не считает и это чтение отвергает.
    unsafe { (vt.add(1).read(), vt.add(2).read()) }
}

/// Слово 0 таблицы: `drop_in_place` или ноль, если деструктора нет.
pub fn raw_has_drop(s: &dyn Sensor) -> bool {
    let (_, vt) = split(s);
    // SAFETY: как в raw_size_align.
    unsafe { vt.read() != 0 }
}
// ANCHOR_END: raw

// ANCHOR: std
/// То же через стабильные функции: компилятор сам читает таблицу.
pub fn std_size_align(s: &dyn Sensor) -> (usize, usize) {
    (size_of_val(s), align_of_val(s))
}

pub fn std_layout(s: &dyn Sensor) -> Layout {
    Layout::for_value(s)
}
// ANCHOR_END: std
