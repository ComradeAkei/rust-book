//! Глава 28: системное программирование под Linux.
//!
//! - [`layers`] — 28.1: один системный вызов через std, libc, rustix и nix.
//! - [`fd`] — 28.2: `OwnedFd`, `BorrowedFd`, `AsFd`, memfd.
//! - [`process`] — 28.3: posix_spawn и fork за `Command`, статус выхода.
//! - [`signals`] — 28.4: обработчик-флаг и signalfd.
//! - [`mm`] — 28.5: mmap файла и анонимной памяти, страничные сбои.
//! - [`poll`] — 28.6: epoll, уровневый и фронтовой режимы.
//! - [`procfs`] — 28.7: разбор /proc/self/maps, status и списков процессоров.
//! - [`journal`] — 28.8: журнал кадров станции в файле с MAP_SHARED.
#![cfg(target_os = "linux")]

pub mod fd;
pub mod journal;
pub mod layers;
pub mod mm;
pub mod poll;
pub mod process;
pub mod procfs;
pub mod signals;
