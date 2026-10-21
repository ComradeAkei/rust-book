//! Типы для задачи 6.A «Предскажите size_of и align_of».
//!
//! Студент не меняет этот файл: он заполняет таблицу ответов в
//! `exercises/ch06/src/a_predict.rs`, а общий тест сравнивает её с
//! настоящими значениями для x86_64.

#![allow(dead_code)]

use core::cell::Cell;
use core::num::NonZeroU32;

/// То же, что `(u8, u32, u8)`, но с раскладкой C.
#[repr(C)]
pub struct Mixed {
    a: u8,
    b: u32,
    c: u8,
}

/// Та же тройка полей без выравнивания.
#[repr(C, packed)]
pub struct Packed {
    a: u8,
    b: u32,
    c: u8,
}

/// Перечисление только с пустыми вариантами.
pub enum Status {
    Idle,
    Busy,
    Error,
}

/// Перечисление, где данные есть не у всех вариантов.
pub enum Shape {
    Point,
    Circle(f32),
    Rect { w: u16, h: u16 },
}

/// Описание одного вопроса задачи: номер, текст типа и настоящие значения.
pub struct Question {
    pub ty: &'static str,
    pub size: usize,
    pub align: usize,
}

macro_rules! questions {
    ($($t:ty),* $(,)?) => {
        /// Настоящие ответы для текущей цели. Тест сравнивает их с таблицей студента.
        pub const QUESTIONS: &[Question] = &[
            $(Question {
                ty: stringify!($t),
                size: core::mem::size_of::<$t>(),
                align: core::mem::align_of::<$t>(),
            }),*
        ];
    };
}

// ANCHOR: list
questions!(
    Option<Option<bool>>,   // 1
    Result<(), Box<u8>>,    // 2
    Option<char>,           // 3
    Option<u32>,            // 4
    Option<NonZeroU32>,     // 5
    (u8, u32, u8),          // 6
    Mixed,                  // 7
    Packed,                 // 8
    Status,                 // 9
    Shape,                  // 10
    Option<*const u8>,      // 11
    Option<Box<dyn Fn()>>,  // 12
    Option<Cell<bool>>,     // 13
    [u64; 0],               // 14
    Option<[bool; 3]>,      // 15
);
// ANCHOR_END: list

// `Box` нужен только в списке типов: size_of не создаёт значений, поэтому
// аллокатор крейту не требуется даже в no_std.
extern crate alloc;
use alloc::boxed::Box;
