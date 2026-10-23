//! Решения задач главы 22. Условия — в разделе 22.7, разбор — в
//! приложении E.
//!
//! - A: размеры future шести `async fn` — предсказание и замер;
//! - B: однопоточный исполнитель с таймерами на виртуальном времени;
//! - C: реактор на epoll и эхо-сервер (только Linux x86-64) и для
//!   сравнения — сервер по потоку на соединение;
//! - D1: исполнитель без кучи на арене, задачи будят прерывания;
//! - D2: отменобезопасное чтение кадров.
//!
//! A и D1 — без std: их собирает и прошивка `firmware/ch22-tasks`.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod a_sizes;
#[cfg(feature = "std")]
pub mod b_timers;
#[cfg(all(feature = "std", target_os = "linux", target_arch = "x86_64"))]
pub mod c_epoll;
pub mod d1_static;
#[cfg(feature = "std")]
pub mod d2_cancel;
