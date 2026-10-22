//! Заготовки задач главы 2. Условия задач в книге, раздел 2.7.
//!
//! Проверка: `cargo test -p ch02-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch02` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Модуль `d_deadlock` компилируется дважды: здесь в edition 2024 и в
//! крейте `ch02-exercises-2021` через `#[path]`. Задачу D проверяют обе
//! команды:
//!
//! ```text
//! cargo test -p ch02-exercises task_d
//! cargo test -p ch02-exercises-2021
//! ```
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code)]

pub mod a_tables;
pub mod b_thumb;
pub mod c_lexer;
pub mod c_regex;
pub mod d_deadlock;

/// Редакция, в которой собран крейт: от неё зависят ожидания задачи D.
pub const EDITION: u32 = 2024;
