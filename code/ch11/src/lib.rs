//! Примеры главы 11 «Динамическая диспетчеризация».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M: без
//! возможности `std` она живёт в `no_std`. Функции, которые глава разбирает
//! в ассемблере, помечены `#[unsafe(no_mangle)]` и, где нужно,
//! `#[inline(never)]`. Фрагменты для книги отмечены парами
//! `ANCHOR: имя` / `ANCHOR_END: имя`. Программы, которые компилятор
//! отвергает, лежат в doc-тестах `compile_fail` с кодом ошибки.
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod any;
pub mod closed;
pub mod compat;
pub mod cost;
pub mod fat;
pub mod object;
pub mod vtable;
