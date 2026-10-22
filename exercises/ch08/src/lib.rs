//! Заготовки задач главы 8. Условия задач в книге, раздел 8.9.
//!
//! Проверка: `cargo test -p ch08-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch08` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Библиотека должна собираться и под Cortex-M0 (`thumbv6m-none-eabi`),
//! поэтому она `no_std`. Задача B — это ещё и `build.rs`, задача C — ещё и
//! библиотека `c/station` с программой на C и `Makefile`, задача D — рабочее
//! пространство `d-station`.
#![no_std]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]

extern crate alloc;

pub mod a_visibility;
pub mod b_sine;
pub mod c_mangle;
