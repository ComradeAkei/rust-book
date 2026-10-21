//! Примеры главы 6 «Составные типы и раскладка памяти».
//!
//! Крейт собирается и для хоста, и для `thumbv6m-none-eabi`/`thumbv7m-none-eabi`,
//! поэтому библиотека живёт в `no_std`, а `std` подключается только в тестах.
//! Каждый модуль соответствует разделу главы, фрагменты для книги
//! отмечены парами `ANCHOR: имя` / `ANCHOR_END: имя`.
#![cfg_attr(not(test), no_std)]

pub mod asm_demo;
pub mod cache;
pub mod enums;
pub mod layout;
pub mod newtype;
pub mod niche;
pub mod packed;
#[cfg(feature = "alloc")]
pub mod task_a;
pub mod task_d;
pub mod unions;
pub mod zst;
