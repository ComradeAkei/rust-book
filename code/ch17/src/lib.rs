//! Примеры главы 17 «Низкоуровневая работа с памятью».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M: без
//! возможности `std` она живёт в `no_std`, а типы динамического размера с
//! хвостовым массивом требуют `alloc` (кучи). Операции с указателями,
//! `MaybeUninit`, `volatile` и `NonNull` не нужна куча — они есть и на
//! голом Cortex-M.
//!
//! Функции, которые глава разбирает в ассемблере, помечены
//! `#[unsafe(no_mangle)]` и `#[inline(never)]`. Фрагменты для книги
//! отмечены парами `ANCHOR: имя` / `ANCHOR_END: имя`. Программы, которые
//! компилятор отвергает, лежат в doc-тестах `compile_fail` с кодом ошибки.
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod codegen;
#[cfg(feature = "alloc")]
pub mod dst;
pub mod maybe_uninit;
pub mod nonnull;
pub mod ptr_ops;
pub mod transmute;
pub mod volatile;
