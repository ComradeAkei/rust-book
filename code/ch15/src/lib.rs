//! Примеры главы 15 «Коллекции и аллокация».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M: без
//! возможности `std` она живёт в `no_std`, а без `alloc` в ней остаются
//! только коллекции `heapless` фиксированной ёмкости. `HashMap` есть лишь в
//! `std`: ей нужен случайный ключ хеширования от операционной системы.
//! Функции, которые глава разбирает в ассемблере, помечены
//! `#[unsafe(no_mangle)]` и `#[inline(never)]`. Фрагменты для книги
//! отмечены парами `ANCHOR: имя` / `ANCHOR_END: имя`. Программы, которые
//! компилятор отвергает, лежат в doc-тестах `compile_fail` с кодом ошибки.
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod alloc_api;
pub mod codegen;
#[cfg(feature = "std")]
pub mod locality;
#[cfg(feature = "std")]
pub mod maps;
pub mod nostd;
#[cfg(feature = "alloc")]
pub mod ordered;
#[cfg(feature = "alloc")]
pub mod strings;
#[cfg(feature = "alloc")]
pub mod tricks;
#[cfg(feature = "alloc")]
pub mod vec;
