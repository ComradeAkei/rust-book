//! Приложение A «Rust для программиста на C: словарь соответствий».
//!
//! Каждое утверждение приложения о поведении программы проверено тестом в
//! одном из модулей ниже. Фрагменты для книги отмечены парами
//! `ANCHOR: имя` / `ANCHOR_END: имя`. C-код, с которым сравнивается Rust,
//! лежит в каталоге `c/` и проверяется скриптом `c/run.sh`.
//!
//! Без возможности `std` крейт собирается под Cortex-M: так проверки при
//! компиляции из `types` сверяют размеры C-типов на ARM.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod asm_demo;
#[cfg(feature = "std")]
pub mod control;
#[cfg(feature = "std")]
pub mod errors;
pub mod funcs;
pub mod ints;
#[cfg(feature = "std")]
pub mod memory;
pub mod preproc;
pub mod strings;
#[cfg(feature = "std")]
pub mod threads;
pub mod types;
pub mod ub;
