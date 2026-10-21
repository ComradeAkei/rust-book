//! Заготовки задач главы 6. Условия задач в книге, раздел 6.10.
//!
//! Проверка: `cargo test -p ch06-exercises`. Пока функции не реализованы,
//! тесты падают на `todo!()`: это нормально. Эталонные решения лежат в
//! `solutions/ch06` и разобраны в приложении E, но сначала попробуйте сами.
#![cfg_attr(not(test), no_std)]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, unused_imports, dead_code)]

pub mod a_predict;
pub mod b_headers;
pub mod c_register;
pub mod d_command;
