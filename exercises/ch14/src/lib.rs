//! Заготовки задач главы 14. Условия задач в книге, раздел 14.9.
//!
//! Проверка: `cargo test -p ch14-exercises`, а для задач B и C ещё и
//! `cargo +nightly miri test -p ch14-exercises`. Пока задачи не решены,
//! тесты падают: это нормально. Эталонные решения лежат в `solutions/ch14`
//! и разобраны в приложении E, но сначала попробуйте сами.
//!
//! Без возможности `std` библиотека собирается в `no_std` для Cortex-M,
//! но с кучей: задачам A, B и C нужен `alloc`, задаче D — потоки.
#![cfg_attr(not(feature = "std"), no_std)]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]

extern crate alloc;

pub mod a_predict;
pub mod b_rc;
pub mod c_list;
#[cfg(feature = "std")]
pub mod d_arc;
