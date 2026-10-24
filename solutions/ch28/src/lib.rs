//! Глава 28: эталонные решения задач.
//!
//! - [`a_strace`] — A: системные вызовы шести операций std под strace.
//! - [`b_ls`] — B: `ls -la` на getdents64 через rustix, без `std::fs`.
//! - [`c_shell`] — C: мини-оболочка с конвейерами, перенаправлениями,
//!   фоновыми заданиями и группами процессов.
//! - [`d_search`] — D: поиск строки через `read` и через `mmap`.
#![cfg(target_os = "linux")]

pub mod a_strace;
pub mod b_ls;
pub mod c_shell;
pub mod d_search;
