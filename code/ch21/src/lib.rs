//! Глава 21: атомики и модель памяти. Как общаться между потоками без замка —
//! атомарными переменными — и что при этом гарантирует порядок операций в
//! памяти.
//!
//! - 21.1 атомарные типы и операции: load/store/swap/fetch_*/compare_exchange
//!   (`atomics`);
//! - 21.2 Release/Acquire и happens-before: передача данных через флаг
//!   (`ordering`);
//! - 21.3 Relaxed, SeqCst и барьеры: буфер записи и когда нужен SeqCst
//!   (`fences`);
//! - 21.5 lock-free SPSC-очередь на атомарных load/store (`spsc`).
//!
//! Модель разбирается на x86-64 с потоками std. SPSC-очередь — `no_std` и
//! строится под Cortex-M; остальное под `--no-default-features` отключается.
#![cfg_attr(not(feature = "std"), no_std)]

// SPSC-очередь обходится атомарными load/store (без RMW), поэтому строится и
// под Cortex-M0, и в no_std.
pub mod spsc;

#[cfg(feature = "std")]
pub mod atomics;
#[cfg(feature = "std")]
pub mod fences;
#[cfg(feature = "std")]
pub mod ordering;
