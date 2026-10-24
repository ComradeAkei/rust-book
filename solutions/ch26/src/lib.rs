//! Глава 26: эталонные решения.
//!
//! - [`a_homes`] — A: где живут десять типов и функций — в core, alloc или
//!   только в std, — и есть ли они на Cortex-M0.
//! - [`b_tiny`] — B: путь к программе `tiny/` — статический ELF для Linux
//!   x86-64 без std и libc.
//! - [`d_mem`] — D: свои `memcpy`, `memmove`, `memset` и их варианты
//!   ARM EABI.
//!
//! Задача C — прошивка `firmware/ch26-boot`: таблица векторов, код запуска
//! и скрипт компоновки без cortex-m-rt.
// ANCHOR: attrs
#![no_std]
// LLVM не заменяет циклы в этом крейте вызовами memcpy и memset: иначе
// memset, написанный циклом, позвал бы сам себя (задача D).
#![no_builtins]
// ANCHOR_END: attrs

pub mod a_homes;
pub mod b_tiny;
pub mod d_mem;
