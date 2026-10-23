//! Эталонные решения задач главы 12. Разбор — в приложении E.
//!
//! Без возможности `std` библиотека собирается в `no_std`: задача A
//! сверяет ответы на `thumbv7m-none-eabi`. Задаче D нужны потоки, и она
//! есть только с `std`.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod a_sizes;
pub mod b_registry;
pub mod c_parser;
#[cfg(feature = "std")]
pub mod d_send;
