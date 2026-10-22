//! Заготовки задач главы 10. Условия задач в книге, раздел 10.8.
//!
//! Проверка: `cargo test -p ch10-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch10` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Библиотека должна собираться и под Cortex-M4F
//! (`thumbv7em-none-eabihf`), поэтому она `no_std`.
#![no_std]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]

pub mod a_copies;
pub mod b_sort;
pub mod c_matrix;
pub mod d_flash;
