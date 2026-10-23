//! Глава 19: встроенный ассемблер, интринсики и SIMD. Как опуститься до
//! конкретных инструкций — через `asm!`, `naked_asm!`, интринсики
//! `core::arch` и SIMD — и как за это отвечать.
//!
//! Ядро главы (asm!, naked-функции, интринсики) — для x86-64 хоста;
//! инструкции управления ядром — для Cortex-M (`#[cfg(target_arch = "arm")]`,
//! проверяются сборкой под thumb). Выбор реализации по фичам процессора
//! требует std (`is_x86_feature_detected!`), переносимый SIMD — nightly.
//! Крейт линкуется с конкретной архитектурой, поэтому под Miri не идёт.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(portable_simd))]

#[cfg(target_arch = "x86_64")]
pub mod asm;
#[cfg(target_arch = "arm")]
pub mod cortex;
#[cfg(all(target_arch = "x86_64", feature = "std"))]
pub mod dispatch;
#[cfg(target_arch = "x86_64")]
pub mod naked;
#[cfg(all(feature = "nightly", target_arch = "x86_64"))]
pub mod portable;
#[cfg(target_arch = "x86_64")]
pub mod simd;
