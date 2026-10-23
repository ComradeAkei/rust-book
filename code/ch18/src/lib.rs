//! Глава 18: FFI с C. Примеры вызова C из Rust (блоки `extern`, `#[repr(C)]`,
//! строки, владение через границу, обратные вызовы, обёртка над zlib) и
//! экспорта Rust в C (раздел 18.4, непрозрачный описатель).
//!
//! Крейт хостовый: он линкуется с C — нашим `c/support.c`, libc и zlib,
//! — поэтому под Miri (Miri не исполняет чужой код) и под Cortex-M не идёт.
//! Экспортированные в C функции проверяет C-программа `c/run.sh`.

pub mod abi;
pub mod callback;
pub mod layout;
pub mod owned;
pub mod strings;
pub mod zlib;
