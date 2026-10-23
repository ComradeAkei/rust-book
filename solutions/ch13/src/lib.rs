//! Эталонные решения задач главы 13. Разбор — в приложении E.
//!
//! Без возможности `alloc` библиотека собирается в `no_std` для Cortex-M:
//! решету простых из задачи B и подсчёту аллокаций из задачи D нужна куча.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod a_chains;
pub mod b_iters;
pub mod c_dot;
pub mod d_vector;
