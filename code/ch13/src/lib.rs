//! Примеры главы 13 «Итераторы».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M: без
//! возможности `std` она живёт в `no_std`, а без `alloc` в ней остаются
//! только итераторы, которым не нужна куча. Функции, которые глава
//! разбирает в ассемблере, помечены `#[unsafe(no_mangle)]` и
//! `#[inline(never)]`. Фрагменты для книги отмечены парами `ANCHOR: имя` /
//! `ANCHOR_END: имя`. Программы, которые компилятор отвергает, лежат в
//! doc-тестах `compile_fail` с кодом ошибки.
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod adapters;
pub mod basics;
pub mod codegen;
pub mod consumers;
pub mod custom;
pub mod tricks;
