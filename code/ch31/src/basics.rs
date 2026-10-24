//! Раздел 31.1: тесты в cargo — модульные, интеграционные и doc-тесты.

use core::fmt;
use core::num::ParseIntError;
use core::str::FromStr;

// ANCHOR: window
/// Скользящее окно последних `N` измерений датчика: среднее за O(1).
///
/// ```
/// use ch31::basics::Window;
///
/// let mut w = Window::<4>::new();
/// assert_eq!(w.mean(), None);
/// for v in [10, 20, 30, 40, 50] {
///     w.push(v);
/// }
/// // В окне 20, 30, 40, 50: первое измерение вытеснено.
/// assert_eq!(w.mean(), Some(35));
/// assert_eq!(w.latest(0), 50);
/// ```
///
/// Окно на ноль измерений не собирается: проверка `N > 0` выполняется
/// при компиляции.
///
/// ```compile_fail,E0080
/// let w = ch31::basics::Window::<0>::new();
/// ```
#[derive(Clone, Debug)]
pub struct Window<const N: usize> {
    buf: [i32; N],
    /// Слот, в который ляжет следующее измерение.
    next: usize,
    len: usize,
    /// Сумма измерений в окне. В i64 не переполняется, пока N < 2³².
    sum: i64,
}

impl<const N: usize> Window<N> {
    const NONEMPTY: () = assert!(N > 0, "окно на ноль измерений");

    pub const fn new() -> Self {
        let () = Self::NONEMPTY;
        Window { buf: [0; N], next: 0, len: 0, sum: 0 }
    }

    pub fn push(&mut self, v: i32) {
        if self.len == N {
            self.sum -= i64::from(self.buf[self.next]);
        } else {
            self.len += 1;
        }
        self.buf[self.next] = v;
        self.sum += i64::from(v);
        self.next = (self.next + 1) % N;
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Среднее с округлением к нулю; у пустого окна среднего нет.
    pub fn mean(&self) -> Option<i32> {
        let n = i64::try_from(self.len).ok().filter(|&n| n > 0)?;
        // Среднее значений i32 лежит между их минимумом и максимумом.
        Some((self.sum / n) as i32)
    }

    /// Измерение «`age` шагов назад»: `latest(0)` — последнее.
    ///
    /// ```should_panic
    /// let w = ch31::basics::Window::<4>::new();
    /// w.latest(0); // паника: окно пусто
    /// ```
    pub fn latest(&self, age: usize) -> i32 {
        assert!(age < self.len, "в окне {} измерений, а запрошено {age} шагов назад", self.len);
        self.buf[(self.next + N - 1 - age) % N]
    }
}
// ANCHOR_END: window

impl<const N: usize> Default for Window<N> {
    fn default() -> Self {
        Self::new()
    }
}

// ANCHOR: from_str
/// Окно из строки вида `"10,20,30"`: числа по порядку поступления.
impl<const N: usize> FromStr for Window<N> {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, ParseIntError> {
        let mut w = Window::new();
        for part in s.split(',').filter(|p| !p.trim().is_empty()) {
            w.push(part.trim().parse()?);
        }
        Ok(w)
    }
}
// ANCHOR_END: from_str

impl<const N: usize> fmt::Display for Window<N> {
    /// Измерения от старого к новому через запятую — обратное к `from_str`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for age in (0..self.len).rev() {
            let sep = if age + 1 == self.len { "" } else { "," };
            write!(f, "{sep}{}", self.latest(age))?;
        }
        Ok(())
    }
}

// ANCHOR: edition
/// В редакции 2024 `std::env::set_var` — `unsafe fn`: окружение процесса
/// общее для всех потоков, а тесты cargo идут в потоках одного процесса.
///
/// ```compile_fail,E0133
/// std::env::set_var("CH31_DEMO", "1");
/// ```
pub const SET_VAR_IS_UNSAFE: () = ();
// ANCHOR_END: edition

// ANCHOR: unit
#[cfg(test)]
mod tests {
    use super::*;

    /// Модульный тест видит приватные поля: инвариант суммы.
    fn sum_is_consistent<const N: usize>(w: &Window<N>) -> bool {
        // Слоты заполняются с нулевого, поэтому до первого вытеснения
        // заняты ровно первые `len`, а после — все.
        let filled = &w.buf[..w.len];
        w.sum == filled.iter().map(|&v| i64::from(v)).sum::<i64>()
    }

    #[test]
    fn sum_follows_contents() {
        let mut w = Window::<3>::new();
        for v in [5, -7, i32::MAX, i32::MIN, 0, 42] {
            w.push(v);
            assert!(sum_is_consistent(&w), "сумма разошлась после {v}: {w:?}");
        }
    }

    #[test]
    fn mean_of_extremes_fits() {
        let mut w = Window::<2>::new();
        w.push(i32::MAX);
        w.push(i32::MAX);
        assert_eq!(w.mean(), Some(i32::MAX));
        w.push(i32::MIN);
        // (MAX + MIN) / 2 = -1 / 2 = 0 с округлением к нулю.
        assert_eq!(w.mean(), Some(0));
    }

    #[test]
    #[should_panic(expected = "в окне 2 измерений")]
    fn latest_beyond_window_panics() {
        let mut w = Window::<4>::new();
        w.push(1);
        w.push(2);
        w.latest(2);
    }

    #[test]
    fn parse_and_display() -> Result<(), ParseIntError> {
        let w: Window<3> = "1, 2,3,4".parse()?;
        assert_eq!(w.to_string(), "2,3,4");
        assert_eq!(w.mean(), Some(3));
        Ok(())
    }
}
// ANCHOR_END: unit
