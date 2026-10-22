//! Заготовки задач главы 11. Условия задач в книге, раздел 11.8.
//!
//! Проверка: `cargo test -p ch11-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch11` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Библиотека должна собираться и под Cortex-M (задача A сверяет ответы
//! на `thumbv7m-none-eabi`), поэтому она `no_std`. `Box` и `Vec` берутся из
//! `alloc`.
#![no_std]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]

extern crate alloc;

pub mod a_sizes;
pub mod b_vtable;
pub mod c_dispatch;
pub mod d_metadata;
