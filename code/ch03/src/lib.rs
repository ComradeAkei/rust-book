//! Примеры главы 3 «Владение и перемещение».
//!
//! Библиотека собирается и для хоста, и для трёх целей Cortex-M, поэтому
//! живёт в `no_std`, а `std` подключается в тестах, в примерах и в модулях,
//! которым нужен журнал событий. Функции, которые глава разбирает в
//! ассемблере, помечены `#[inline(never)]` и `#[unsafe(no_mangle)]`, как в
//! главах 0–2. Фрагменты для книги отмечены парами `ANCHOR: имя` /
//! `ANCHOR_END: имя`.
#![cfg_attr(not(test), no_std)]

// Журнал событий в `drop_order` и примеры с `Vec` и `Box` живут на хосте.
#[cfg(all(not(test), not(target_os = "none")))]
extern crate std;

#[cfg(not(target_os = "none"))]
pub mod drop_flags;
#[cfg(not(target_os = "none"))]
pub mod drop_order;
pub mod flags;
#[cfg(not(target_os = "none"))]
pub mod mem_fns;
pub mod moves;
#[cfg(not(target_os = "none"))]
pub mod ownership;
pub mod regs;
#[cfg(not(target_os = "none"))]
pub mod task_a;
pub mod task_c;
