//! Примеры главы 14 «Умные указатели и внутренняя изменяемость».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M: без
//! возможности `std` она живёт в `no_std`, а без `alloc` в ней остаются
//! только ячейки. `Arc` есть лишь там, где у процессора есть атомарное
//! чтение-изменение-запись указателя: на Cortex-M0 модуля `alloc::sync` нет.
//! Функции, которые глава разбирает в ассемблере, помечены
//! `#[unsafe(no_mangle)]` и `#[inline(never)]`. Фрагменты для книги
//! отмечены парами `ANCHOR: имя` / `ANCHOR_END: имя`. Программы, которые
//! компилятор отвергает, лежат в doc-тестах `compile_fail` с кодом ошибки.
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod arc;
#[cfg(feature = "alloc")]
pub mod boxes;
pub mod cells;
pub mod codegen;
#[cfg(feature = "alloc")]
pub mod cow;
#[cfg(feature = "alloc")]
pub mod deref;
pub mod lazy;
#[cfg(feature = "alloc")]
pub mod rc;
#[cfg(feature = "alloc")]
pub mod tricks;
