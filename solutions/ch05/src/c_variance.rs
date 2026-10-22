//! Задача 5.C: вариантность десяти типов по параметру `'a`.
//!
//! Части 1 и 2 — файлы `variance/covariant.rs` и `variance/cell.rs`.

pub use Variance::{Co, Contra, Inv};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Variance {
    /// Ковариантен: `T<'long>` годится вместо `T<'short>`.
    Co,
    /// Контравариантен: `T<'short>` годится вместо `T<'long>`.
    Contra,
    /// Инвариантен: годится только `T<'a>` с тем же `'a`.
    Inv,
}

/// Типы задачи; `Cell` — `core::cell::Cell`.
pub const TYPES: [&str; 10] = [
    "&'a u8",
    "&'a mut u8",
    "&'a mut &'a u8",
    "*const &'a u8",
    "*mut &'a u8",
    "Cell<&'a u8>",
    "fn(&'a u8)",
    "fn(fn(&'a u8))",
    "Box<dyn Fn(&'a u8)>",
    "core::str::Lines<'a>",
];

pub const VARIANCE: [Option<Variance>; 10] = [
    Some(Co),     // &'a T ковариантен по 'a
    Some(Co),     // &'a mut T ковариантен по 'a, инвариантен по T
    Some(Inv),    // 'a стоит и внутри T у &mut T
    Some(Co),     // *const T — как &T
    Some(Inv),    // *mut T — как &mut T по T
    Some(Inv),    // Cell<T> инвариантен: запись через &
    Some(Contra), // аргумент функции
    Some(Co),     // аргумент аргумента: два знака минус
    Some(Inv),    // параметры трейта у dyn инвариантны
    Some(Inv),    // внутри Lines — проекция Pattern::Searcher<'a>
];
