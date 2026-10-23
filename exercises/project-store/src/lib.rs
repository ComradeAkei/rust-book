//! Шаг главы 15 (задача П15): хранилище измерений на хосте и
//! аллокатор-счётчик. ЗАГОТОВКА: типы и конструкторы даны, тела методов и
//! подсчёт выделений — `todo!()`. Проверка: `cargo test -p project-store
//! p15`.
//!
//! Условие — в разделе [15.11](../../../src/ch15/11-project.md) учебника.

pub mod alloc;
pub mod store;

pub use alloc::{Counted, Counts};
pub use store::{Ingested, Point, Store, Summary};
