//! Примеры главы 7 «Обработка ошибок и паника».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M: без фичи
//! `std` она живёт в `no_std`, `Box` и `String` берутся из `alloc`, а
//! трейт `Error` — из `core::error`. Функции, которые глава разбирает в
//! ассемблере, помечены `#[inline(never)]` и `#[unsafe(no_mangle)]`.
//! Фрагменты для книги отмечены парами `ANCHOR: имя` / `ANCHOR_END: имя`.
//! Паника, раскрутка и граница FFI показаны программой `src/bin/panics.rs`:
//! часть опытов завершает процесс через abort, поэтому тесты запускают её
//! отдельным процессом.
#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;

pub mod abi;
pub mod bus;
pub mod chain;
pub mod combinators;
pub mod convert;
#[cfg(feature = "std")]
pub mod io;
pub mod sizes;
pub mod unwind;
