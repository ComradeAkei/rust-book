//! Эталонные решения задач главы 5. Разборы — в приложении E.
//!
//! Проверка: `cargo test -p ch05-solutions`. Части задач C и D лежат вне
//! крейта, в `variance/` и `windows/`: тесты собирают их сами.
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod a_elision;
pub mod b_lexer;
pub mod c_variance;
pub mod d_windows;
