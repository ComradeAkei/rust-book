//! Заготовки задач главы 15. Условия задач в книге, раздел 15.10.
//!
//! Проверка: `cargo test -p ch15-exercises`, а для задачи C ещё и
//! `cargo +nightly miri test -p ch15-exercises --test ch15`. Пока задачи
//! не решены, тесты падают: это нормально. Эталонные решения лежат в
//! `solutions/ch15` и разобраны в приложении E, но сначала попробуйте сами.
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]

pub mod a_predict;
pub mod b_count;
pub mod c_bump;
pub mod d_table;
