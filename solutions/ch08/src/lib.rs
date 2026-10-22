//! Эталонные решения задач главы 8. Разборы — в приложении E.
//!
//! Библиотека собирается и под Cortex-M0 (`thumbv6m-none-eabi`): тест
//! задачи B проверяет, где в объектном коде лежит таблица синуса. Задача C
//! — это ещё и библиотека `c/station` с программой на C, задача D —
//! рабочее пространство `d-station`.
#![no_std]

extern crate alloc;

pub mod a_visibility;
pub mod b_sine;
pub mod c_mangle;
