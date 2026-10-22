//! Эталонные решения задач главы 4.
//!
//! Задача A — ответы о фрагментах из `code/ch04/fragments`. Задача B —
//! функция на Rust и её аналоги на C в `c/add_twice.c`. Задача C — граф
//! сборки на арене (и рекурсивный обход для сравнения). Задача D — обходные пути; программы, которые проверка
//! заимствований отвергает, лежат в `polonius/` и в крейт не входят.
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod a_borrowck;
pub mod b_add_twice;
pub mod c_build_graph;
pub mod c_recursive;
pub mod d_polonius;
