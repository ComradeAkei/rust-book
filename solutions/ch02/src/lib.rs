//! Эталонные решения задач главы 2.
//!
//! Модуль `d_deadlock` компилируется дважды: здесь в edition 2024 и в
//! крейте `ch02-solutions-2021` через `#[path]`, чтобы тесты увидели
//! разницу редакций на одном исходнике.

pub mod a_tables;
pub mod b_thumb;
pub mod c_lexer;
pub mod c_regex;
pub mod d_deadlock;

/// Редакция, в которой собран крейт: от неё зависят ожидания задачи D.
pub const EDITION: u32 = 2024;
