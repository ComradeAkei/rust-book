//! Заготовки задач главы 22. Условия — в разделе 22.7.
//!
//! - A: размеры future шести `async fn` — предскажите, затем сверьте.
//! - B: однопоточный исполнитель с таймерами на виртуальном времени.
//! - C: реактор на epoll и эхо-сервер на одном потоке (Linux x86-64).
//! - D1: исполнитель без кучи на арене, задачи будят прерывания.
//! - D2: отменобезопасное чтение кадров.
//!
//! Проверка: `cargo test -p ch22-exercises`. Пока задачи не решены, тесты
//! падают: это нормально. Эталонные решения лежат в `solutions/ch22` и
//! разобраны в приложении E, но сначала попробуйте сами.
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables, dead_code, unused_imports)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod a_sizes;
#[cfg(feature = "std")]
pub mod b_timers;
#[cfg(all(feature = "std", target_os = "linux", target_arch = "x86_64"))]
pub mod c_epoll;
pub mod d1_static;
#[cfg(feature = "std")]
pub mod d2_cancel;
