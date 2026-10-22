//! Задача C: матрицы, размерности которых — параметры-константы. Умножить
//! 2×3 на 2×3 нельзя: это ошибка типов, а не проверка во время работы.

use core::ops::{Add, Mul};

// ANCHOR: matrix
/// Матрица `R × C` целых. Арифметика с переполнением по модулю 2³², как у
/// целочисленного DSP-кода: результат не зависит от сборки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matrix<const R: usize, const C: usize>(pub [[i32; C]; R]);

impl<const R: usize, const C: usize> Matrix<R, C> {
    pub const ZERO: Self = Matrix([[0; C]; R]);

    /// Матрица из функции индексов: `m[i][j] = f(i, j)`.
    pub fn from_fn(mut f: impl FnMut(usize, usize) -> i32) -> Self {
        Matrix(core::array::from_fn(|i| core::array::from_fn(|j| f(i, j))))
    }

    /// `C × R`: размерности меняются местами и в типе.
    pub fn transpose(&self) -> Matrix<C, R> {
        Matrix::from_fn(|i, j| self.0[j][i])
    }

    /// Произведение `R × C` на `C × K`. Внутренние размерности совпадают
    /// по построению: другой `rhs` не пройдёт проверку типов.
    pub fn matmul<const K: usize>(&self, rhs: &Matrix<C, K>) -> Matrix<R, K> {
        let mut out = Matrix::<R, K>::ZERO;
        for i in 0..R {
            for k in 0..K {
                let mut s = 0i32;
                for j in 0..C {
                    s = s.wrapping_add(self.0[i][j].wrapping_mul(rhs.0[j][k]));
                }
                out.0[i][k] = s;
            }
        }
        out
    }

    /// Произведение на вектор-столбец длины `C`.
    pub fn apply(&self, v: [i32; C]) -> [i32; R] {
        core::array::from_fn(|i| {
            self.0[i].iter().zip(v).fold(0i32, |s, (&a, b)| s.wrapping_add(a.wrapping_mul(b)))
        })
    }
}

/// Только у квадратных матриц есть единичная матрица и след.
impl<const N: usize> Matrix<N, N> {
    pub const fn identity() -> Self {
        let mut m = [[0; N]; N];
        let mut i = 0;
        while i < N {
            m[i][i] = 1;
            i += 1;
        }
        Matrix(m)
    }

    pub fn trace(&self) -> i32 {
        (0..N).fold(0i32, |s, i| s.wrapping_add(self.0[i][i]))
    }
}

impl<const R: usize, const C: usize> Add for Matrix<R, C> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Matrix::from_fn(|i, j| self.0[i][j].wrapping_add(rhs.0[i][j]))
    }
}

/// `a * b` — то же, что `a.matmul(&b)`.
impl<const R: usize, const C: usize, const K: usize> Mul<Matrix<C, K>> for Matrix<R, C> {
    type Output = Matrix<R, K>;

    fn mul(self, rhs: Matrix<C, K>) -> Matrix<R, K> {
        self.matmul(&rhs)
    }
}
// ANCHOR_END: matrix

// ANCHOR: sizes
/// Пять размеров для замера кода: у каждого своя копия `matmul`.
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
