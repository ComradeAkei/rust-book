//! Примеры главы 26 «no_std: программа без операционной системы».
//!
//! - [`layers`] — 26.1: один отчёт на трёх слоях: в буфер на стеке
//!   (`core`), в `String` (`alloc`), в stdout (`std`).
//! - [`sys`] — 26.2: системные вызовы Linux на x86-64 без libc. Ими
//!   пользуется программа `code/ch26/start` со своим `_start`.
//! - [`copies`] — 26.5: обычный код, из которого компилятор делает
//!   вызовы `memcpy`, `memset`, `memcmp` и `strlen`.
//!
//! Крейт `no_std` всегда; `alloc` и `std` подключаются возможностями
//! с теми же именами. Без возможностей по умолчанию он собирается для
//! Cortex-M, и его функции из `copies` вызывает прошивка
//! `firmware/ch26-bare`.
// ANCHOR: crate
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;
// ANCHOR_END: crate

pub mod copies;
pub mod layers;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub mod sys;
