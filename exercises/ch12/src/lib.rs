//! Заготовки задач главы 12. Условия задач в книге, раздел 12.7.
//!
//! Проверка: `cargo test -p ch12-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch12` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Без возможности `std` библиотека собирается в `no_std`: задача A
//! сверяет ответы на `thumbv7m-none-eabi`. Задаче D нужны потоки, и она
//! есть только с `std`.
#![cfg_attr(not(feature = "std"), no_std)]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]

pub mod a_sizes;
pub mod b_registry;
pub mod c_parser;
#[cfg(feature = "std")]
pub mod d_send;
