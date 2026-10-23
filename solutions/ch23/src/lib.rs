//! Глава 23: эталонные решения задач.
//!
//! - [`a_expand`] — A: предсказанные раскрытия шести вызовов.
//! - [`b_flags`] — B: [`flags!`], флаги регистра в духе `bitflags`.
//! - [`c_periph`] — C: [`peripheral!`], описание периферии с правами
//!   доступа.
//! - [`d_fsm`] — D: [`fsm!`], конечный автомат на TT-muncher.
//!
//! Макросы экспортированы (`#[macro_export]`) и лежат в корне крейта:
//! `ch23_solutions::flags!`. Сгенерированный ими код — `no_std`, его
//! использует прошивка `firmware/ch23-periph`.
#![no_std]

pub mod a_expand;
pub mod b_flags;
pub mod c_periph;
pub mod d_fsm;
