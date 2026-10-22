//! Примеры главы 4 «Заимствование и ссылки».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M, поэтому
//! живёт в `no_std`; `Vec`, `String` и `Box` берутся из `alloc`. Функции,
//! которые глава разбирает в ассемблере, помечены `#[inline(never)]` и
//! `#[unsafe(no_mangle)]`. Фрагменты для книги отмечены парами
//! `ANCHOR: имя` / `ANCHOR_END: имя`.
//!
//! Фрагменты задачи 4.A лежат отдельно, в `code/ch04/fragments`: половина
//! из них не компилируется нарочно.
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod arena;
pub mod borrows;
pub mod deref;
pub mod limits;
pub mod noalias;
pub mod refs;
pub mod split;
