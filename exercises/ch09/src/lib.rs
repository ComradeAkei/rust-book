//! Заготовки задач главы 9. Условия задач в книге, раздел 9.8.
//!
//! Проверка: `cargo test -p ch09-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch09` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Библиотека должна собираться и под Cortex-M0 (`thumbv6m-none-eabi`),
//! поэтому она `no_std`.
#![no_std]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]

pub mod a_coherence;
pub mod b_register;
pub mod c_sealed;
pub mod d_bus;
