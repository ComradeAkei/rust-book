//! Примеры главы 1 «Скалярные типы и представление в памяти».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M, поэтому
//! живёт в `no_std`, а `std` подключается только в тестах и примерах.
//! Функции, которые глава разбирает в ассемблере, помечены
//! `#[inline(never)]` и `#[unsafe(no_mangle)]`, как в главе 0. Фрагменты для
//! книги отмечены парами `ANCHOR: имя` / `ANCHOR_END: имя`.
#![cfg_attr(not(test), no_std)]

pub mod bits;
pub mod casts;
pub mod chars;
pub mod floats;
pub mod ints;
pub mod overflow;
pub mod slices;
pub mod task_a;
pub mod wide;
