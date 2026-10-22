//! Эталонные решения задач главы 10. Разбор — в приложении E.
//!
//! Библиотека собирается и под Cortex-M4F (`thumbv7em-none-eabihf`):
//! матрицы задачи C меряются на ней. Задача B объявляет `qsort` из libc,
//! но связывается с ней только программа для хоста.
#![no_std]

pub mod a_copies;
pub mod b_sort;
pub mod c_matrix;
pub mod d_flash;
