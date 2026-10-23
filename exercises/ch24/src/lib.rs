//! Глава 24: заготовки задач для студента.
//!
//! - [`a_expand`] — A: раскрытие `#[derive(Debug)]` и `#[entry]` из прошивки
//!   `firmware/ch24-expand`.
//! - [`b_bitfield`] — B: [`Bitfield`], поля регистра с `#[bits(N)]`.
//! - [`c_timed`] — C: [`timed`], замер времени функции; часы для хоста и
//!   Cortex-M.
//! - [`d_svd`] — D: [`include_svd!`], регистры периферии из файла SVD.
//!
//! Макросы — ваша часть работы — живут в крейте `ch24-exercises-macros`
//! (`exercises/ch24-macros`) и реэкспортируются отсюда. Счётчик, охранник
//! и часы задачи C даны готовыми, как в решениях.
#![cfg_attr(not(any(test, feature = "std")), no_std)]

pub use tasks_macros::{Bitfield, include_svd, timed};

pub mod a_expand;
pub mod b_bitfield;
pub mod c_timed;
pub mod d_svd;
