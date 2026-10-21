//! Заготовки задач главы 1. Условия задач в книге, раздел 1.8.
//!
//! Проверка: `cargo test -p ch01-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch01` и
//! разобраны в приложении E, но сначала попробуйте сами.
//!
//! Крейт собирается и для хоста, и для Cortex-M, чтобы ассемблер задач B1
//! и D можно было снять под обе архитектуры. Задачам C и D нужна `std`
//! (`catch_unwind` и `f32::mul_add`), и на целях без ОС их модули не
//! собираются; D там доступна на nightly с `--features nightly`.
#![no_std]
#![cfg_attr(feature = "nightly", feature(core_float_math))]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code)]

#[cfg(not(target_os = "none"))]
extern crate std;

pub mod a_casts;
pub mod b1_u128;
pub mod b2_absorb;
#[cfg(not(target_os = "none"))]
pub mod c_probe;
#[cfg(any(not(target_os = "none"), feature = "nightly"))]
pub mod d_fma;
