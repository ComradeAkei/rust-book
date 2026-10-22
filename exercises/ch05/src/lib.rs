//! Заготовки задач главы 5. Условия задач в книге, раздел 5.8.
//!
//! Проверка: `cargo test -p ch05-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch05` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Задача C — это ещё и файлы в `variance/`, задача D — ещё и
//! `windows/iterator.rs`: они в крейт не входят, тесты собирают их сами.
// Параметры и поля заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod a_elision;
pub mod b_lexer;
pub mod c_variance;
pub mod d_windows;
