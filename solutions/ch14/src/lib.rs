//! Эталонные решения задач главы 14. Условия задач в книге, раздел 14.9,
//! разборы — в приложении E.
//!
//! Без возможности `std` библиотека собирается в `no_std` для Cortex-M,
//! но с кучей: задачам A, B и C нужен `alloc`, задаче D — потоки.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod a_predict;
pub mod b_rc;
pub mod c_list;
#[cfg(feature = "std")]
pub mod d_arc;
