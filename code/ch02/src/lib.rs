//! Примеры главы 2 «Выражения, управление потоком, сопоставление с
//! образцом».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M, поэтому
//! живёт в `no_std`, а `std` подключается только в тестах и примерах.
//! Функции, которые глава разбирает в ассемблере, помечены
//! `#[inline(never)]` и `#[unsafe(no_mangle)]`, как в главах 0 и 1.
//! Фрагменты для книги отмечены парами `ANCHOR: имя` / `ANCHOR_END: имя`.
#![cfg_attr(not(test), no_std)]

// Раздел 2.6 разбирает `std::sync::Mutex`, которого в `core` нет.
#[cfg(all(not(test), not(target_os = "none")))]
extern crate std;

pub mod exprs;
pub mod iflet;
pub mod lexer;
pub mod loops;
pub mod lowering;
pub mod patterns;
pub mod task_a;
#[cfg(not(target_os = "none"))]
pub mod temporaries;
pub mod thumb;
