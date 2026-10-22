//! Эталонные решения задач главы 1. Разбор каждого решения с цифрами и
//! ассемблером лежит в приложении E книги.
//!
//! Крейт собирается и для хоста, и для Cortex-M, чтобы ассемблер задач B1
//! и D можно было снять под обе архитектуры. Задачам C и D нужна `std`
//! (`catch_unwind` и `f32::mul_add`), и на целях без ОС их модули не
//! собираются; D там доступна на nightly с `--features nightly`.
#![no_std]
#![cfg_attr(feature = "nightly", feature(core_float_math))]

#[cfg(not(target_os = "none"))]
extern crate std;

pub mod a_casts;
pub mod b1_u128;
pub mod b2_absorb;
#[cfg(not(target_os = "none"))]
pub mod c_probe;
#[cfg(any(not(target_os = "none"), feature = "nightly"))]
pub mod d_fma;
