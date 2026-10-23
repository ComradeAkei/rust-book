//! Эталонные решения задач главы 11. Разбор — в приложении E.
//!
//! Библиотека собирается и под Cortex-M: задача A сверяет ответы на
//! `thumbv7m-none-eabi`. `Box` и `Vec` берутся из `alloc`.
#![no_std]

extern crate alloc;

pub mod a_sizes;
pub mod b_vtable;
pub mod c_dispatch;
pub mod d_metadata;
