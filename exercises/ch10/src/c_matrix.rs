//! Задача C: матрицы, размерности которых — параметры-константы. Сейчас
//! заготовка принимает и несовместимые размерности: `matmul` обобщена по
//! лишнему параметру, а единичная матрица есть и у прямоугольных. Сделайте
//! так, чтобы ошибки размерностей ловил компилятор.

use core::ops::{Add, Mul};

// ANCHOR: matrix
/// Матрица `R × C` целых. Арифметика с переполнением по модулю 2³².
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matrix<const R: usize, const C: usize>(pub [[i32; C]; R]);

impl<const R: usize, const C: usize> Matrix<R, C> {
    pub const ZERO: Self = Matrix([[0; C]; R]);

    /// Матрица из функции индексов: `m[i][j] = f(i, j)`.
    pub fn from_fn(f: impl FnMut(usize, usize) -> i32) -> Self {
        todo!()
    }

    pub fn transpose(&self) -> Matrix<C, R> {
        todo!()
    }

    /// Произведение на `R2 × K`. Совпадение `R2` и `C` пока никто не
    /// проверяет.
    pub fn matmul<const R2: usize, const K: usize>(&self, rhs: &Matrix<R2, K>) -> Matrix<R, K> {
        todo!()
    }

    /// Произведение на вектор-столбец длины `C`.
    pub fn apply(&self, v: [i32; C]) -> [i32; R] {
        todo!()
    }

    /// Единичная матрица. Пока есть и у прямоугольных.
    pub fn identity() -> Self {
        todo!()
    }

    pub fn trace(&self) -> i32 {
        todo!()
    }
}

impl<const R: usize, const C: usize> Add for Matrix<R, C> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        todo!()
    }
}

impl<const R: usize, const C: usize, const K: usize> Mul<Matrix<C, K>> for Matrix<R, C> {
    type Output = Matrix<R, K>;

    fn mul(self, rhs: Matrix<C, K>) -> Matrix<R, K> {
        todo!()
    }
}
// ANCHOR_END: matrix

// ANCHOR: sizes
/// Пять размеров для замера кода.
macro_rules! sized {
    ($($name:ident = $n:literal),*) => {
        $(
            #[unsafe(no_mangle)]
            pub fn $name(a: &Matrix<$n, $n>, b: &Matrix<$n, $n>, out: &mut Matrix<$n, $n>) {
                *out = a.matmul(b);
            }
        )*
    };
}

sized!(ch10c_mul2 = 2, ch10c_mul3 = 3, ch10c_mul4 = 4, ch10c_mul6 = 6, ch10c_mul8 = 8);
// ANCHOR_END: sizes
