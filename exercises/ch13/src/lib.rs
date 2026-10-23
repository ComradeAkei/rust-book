//! Заготовки задач главы 13. Условия задач в книге, раздел 13.7.
//!
//! Проверка: `cargo test -p ch13-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch13` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Без возможности `alloc` библиотека собирается в `no_std` для Cortex-M:
//! решету простых из задачи B и подсчёту аллокаций из задачи D нужна куча.
#![cfg_attr(not(feature = "std"), no_std)]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod a_chains;
pub mod b_iters;
pub mod c_dot;
pub mod d_vector;
