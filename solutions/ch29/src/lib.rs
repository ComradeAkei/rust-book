//! Глава 29: эталонные решения задач.
//!
//! - [`a_bugs`] — A: ошибки шести фрагментов C-драйвера и что с ними
//!   делает перевод на Rust.
//! - B, C и D1 — модули ядра в `modules/`: `ch29_b.rs` (параметр и
//!   dmesg), `ch29_c.rs` (счётчик /dev/ch29-counter), `ch29_d1.rs`
//!   (samples/kobject/kobject-example.c на Rust).
//! - [`d2_trace`] — D2: разбор трассы function_graph и вывода objdump для
//!   замера горячего пути read.
#![cfg(target_os = "linux")]

pub mod a_bugs;
pub mod d2_trace;
