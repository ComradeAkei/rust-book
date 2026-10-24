//! Глава 30: эталонные решения задач.
//!
//! - [`a_bench`] — A: пять замеров с ошибкой методики и исправленные.
//! - [`b_particles`] — B: система частиц строками (AoS) и столбцами (SoA).
//! - [`c_ngrams`] — C: частые n-граммы текста в десять раз быстрее
//!   наивного подсчёта.
//! - [`d_branches`] — D: поиск с ветвлением и без, и где LLVM сам ставит
//!   `cmov`.
//!
//! Замеры — программа `ch30-task` (tests-shared/ch30-task.rs).

pub mod a_bench;
pub mod b_particles;
pub mod c_ngrams;
pub mod d_branches;
