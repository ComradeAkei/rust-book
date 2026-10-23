//! Глава 24: эталонные решения задач.
//!
//! - [`a_expand`] — A: раскрытие `#[derive(Debug)]` и `#[entry]` из прошивки
//!   `firmware/ch24-expand`.
//! - [`b_bitfield`] — B: [`Bitfield`], поля регистра с `#[bits(N)]`.
//! - [`c_timed`] — C: [`timed`], замер времени функции; часы для хоста и
//!   Cortex-M.
//! - [`d_svd`] — D: [`include_svd!`], регистры периферии из файла SVD.
//!
//! Макросы живут в крейте `ch24-solutions-macros` и реэкспортируются
//! отсюда. Без возможности `std` библиотека — `no_std`: её использует
//! прошивка `firmware/ch24-tasks`.
#![cfg_attr(not(any(test, feature = "std")), no_std)]

pub use tasks_macros::{Bitfield, include_svd, timed};

pub mod a_expand;
pub mod b_bitfield;
pub mod c_timed;
pub mod d_svd;
