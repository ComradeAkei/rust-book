//! Заготовки задач главы 3. Условия задач в книге, раздел 3.7.
//!
//! Проверка: `cargo test -p ch03-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch03` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Задачи B и D под Miri:
//!
//! ```text
//! cargo +nightly miri test -p ch03-exercises task_b task_d
//! cargo +nightly miri test -p ch03-exercises -- --ignored --exact task_d::naive_in_place_ub
//! ```
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[cfg(not(target_os = "none"))]
pub mod a_drop_order;
pub mod b_scope_guard;
pub mod c_abi;
pub mod d_self_ref;
