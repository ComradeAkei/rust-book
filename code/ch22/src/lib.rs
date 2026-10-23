//! Примеры главы 22: асинхронность изнутри.
//!
//! - [`future`] — 22.1: `Future`, `poll`, `Context` и `Waker`.
//! - [`state`] — 22.2: `async fn` как конечный автомат и размер future.
//! - [`pin`] — 22.3: самоссылающиеся автоматы, `Pin` и `Unpin`.
//! - [`executor`] — 22.4: исполнитель с очередью задач (std).
//! - [`select`] — 22.5: `select`, `join` и отмена через `drop`.
//! - [`bare`] — 22.6: исполнитель без кучи и без ОС, будильник из прерывания.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod bare;
#[cfg(feature = "std")]
pub mod executor;
pub mod future;
pub mod pin;
pub mod select;
pub mod state;
