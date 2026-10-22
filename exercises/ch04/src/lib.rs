//! Заготовки задач главы 4. Условия задач в книге, раздел 4.7.
//!
//! Проверка: `cargo test -p ch04-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch04` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Задача B — это ещё и `c/add_twice.c`, задача D — ещё и файлы в
//! `polonius/`: они в крейт не входят, тесты собирают их сами. Для
//! проверки Polonius нужен nightly:
//!
//! ```text
//! rustup toolchain install nightly --profile minimal
//! cargo test -p ch04-exercises -- task_d
//! ```
// Параметры заготовок пока не используются, и это ожидаемо. По пустым
// телам clippy советует то, что решению не подходит: срез вместо `&mut Vec`
// в `acquire`, которой нужен `push`, и `Default` для `Graph` без полей.
#![allow(unused_variables, dead_code, unused_imports)]
#![allow(clippy::ptr_arg, clippy::new_without_default)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod a_borrowck;
pub mod b_add_twice;
pub mod c_build_graph;
pub mod d_polonius;
