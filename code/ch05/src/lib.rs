//! Примеры главы 5 «Времена жизни».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M, поэтому
//! живёт в `no_std`; `Vec`, `String` и `Box` берутся из `alloc`. Функции,
//! которые глава разбирает в ассемблере, помечены `#[inline(never)]` и
//! `#[unsafe(no_mangle)]`. Фрагменты для книги отмечены парами
//! `ANCHOR: имя` / `ANCHOR_END: имя`. Программы, которые не компилируются
//! нарочно, записаны в doc-тестах `compile_fail` с кодом ошибки.
#![cfg_attr(not(test), no_std)]
// Глава нарочно выписывает времена жизни, которые правила элизии позволяют
// опустить: так видно, что именно подставляет компилятор.
#![allow(clippy::needless_lifetimes)]

extern crate alloc;

pub mod capture;
pub mod config;
pub mod elision;
pub mod erase;
pub mod gat;
pub mod hrtb;
pub mod statics;
pub mod tlv;
pub mod variance;
