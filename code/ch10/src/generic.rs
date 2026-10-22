//! Раздел 10.1: обобщённые функции и типы, turbofish.
//!
//! Тело обобщённой функции компилятор проверяет один раз, по её
//! ограничениям, а не после подстановки типа, как тело макроса в C. Без
//! `PartialOrd` значения `T` сравнивать нельзя, даже если функцию никто не
//! вызывает:
//!
//! ```compile_fail,E0369
//! pub fn min_of<T: Copy>(v: &[T]) -> Option<T> {
//!     let (&first, rest) = v.split_first()?;
//!     Some(rest.iter().fold(first, |m, &x| if x < m { x } else { m }))
//! }
//! ```
//!
//! Тип, который ограничениям не отвечает, отвергается в месте вызова:
//!
//! ```compile_fail,E0277
//! #[derive(Clone, Copy)]
//! struct Point {
//!     x: i32,
//!     y: i32,
//! }
//! let p = ch10::generic::min_of(&[Point { x: 1, y: 2 }]);
//! ```
//!
//! Тип результата, который ничем не подсказан, нужно назвать: аннотацией
//! переменной или turbofish `::<>`.
//!
//! ```compile_fail,E0283
//! let z = ch10::generic::zeroed();
//! ```
//!
//! ```
//! let z = ch10::generic::zeroed::<[u16; 4]>();
//! assert_eq!(z, [0; 4]);
//! ```

// ANCHOR: min-of
/// Наименьший элемент среза. `T: PartialOrd` разрешает `<`, `T: Copy` —
/// вынуть значение из среза, не забирая его.
pub fn min_of<T: PartialOrd + Copy>(v: &[T]) -> Option<T> {
    let (&first, rest) = v.split_first()?;
    let mut m = first;
    for &x in rest {
        if x < m {
            m = x;
        }
    }
    Some(m)
}
// ANCHOR_END: min-of

// ANCHOR: minmax
/// Наименьшее и наибольшее значение потока отсчётов.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinMax<T> {
    pub min: T,
    pub max: T,
}

impl<T: PartialOrd + Copy> MinMax<T> {
    pub fn new(first: T) -> Self {
        MinMax { min: first, max: first }
    }

    pub fn push(&mut self, x: T) {
        if x < self.min {
            self.min = x;
        }
        if x > self.max {
            self.max = x;
        }
    }

    pub fn of(v: &[T]) -> Option<Self> {
        let (&first, rest) = v.split_first()?;
        let mut m = MinMax::new(first);
        for &x in rest {
            m.push(x);
        }
        Some(m)
    }
}

/// Метод только для `MinMax<i16>`: размах отсчётов АЦП. Разность двух
/// `i16` может не поместиться в `i16`, а в `u16` помещается всегда.
impl MinMax<i16> {
    pub fn span(&self) -> u16 {
        self.max.abs_diff(self.min)
    }
}
// ANCHOR_END: minmax

// ANCHOR: turbofish
/// Значение по умолчанию для любого `T`. Аргументов нет, и вывести `T`
/// компилятор может только из того, куда пойдёт результат.
pub fn zeroed<T: Default>() -> T {
    T::default()
}

/// Разобрать числа через запятую. Тип чисел выбирает вызывающий.
#[cfg(feature = "alloc")]
pub fn parse_list<T: core::str::FromStr>(s: &str) -> Result<alloc::vec::Vec<T>, T::Err> {
    s.split(',').map(|x| x.trim().parse::<T>()).collect()
}
// ANCHOR_END: turbofish

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_functions() {
        assert_eq!(min_of(&[3u8, 1, 2]), Some(1));
        assert_eq!(min_of(&["б", "а"]), Some("а"));
        assert_eq!(min_of::<i32>(&[]), None);
        // NaN не меньше и не больше ничего: PartialOrd, а не Ord.
        assert_eq!(min_of(&[2.0f32, f32::NAN, 1.0]), Some(1.0));
        assert!(min_of(&[f32::NAN, 1.0]).unwrap().is_nan());
    }

    #[test]
    fn generic_types() {
        let m = MinMax::of(&[-300i16, 4000, 12]).unwrap();
        assert_eq!(m, MinMax { min: -300, max: 4000 });
        assert_eq!(m.span(), 4300);
        assert_eq!(MinMax::of(&[i16::MIN, i16::MAX]).unwrap().span(), u16::MAX);
        let f = MinMax::of(&[0.5f32, -0.25]).unwrap();
        assert_eq!((f.min, f.max), (-0.25, 0.5));
    }

    #[test]
    fn turbofish() {
        assert_eq!(zeroed::<u32>(), 0);
        let v: [i8; 2] = zeroed();
        assert_eq!(v, [0, 0]);
        assert_eq!(parse_list::<u8>("1, 2,3"), Ok(alloc::vec![1, 2, 3]));
        assert!(parse_list::<u8>("1,300").is_err());
        assert_eq!(parse_list::<i16>("-300,7"), Ok(alloc::vec![-300, 7]));
        assert_eq!("42".parse::<u8>(), Ok(42));
        assert_eq!([1u8, 2].iter().map(|&x| u32::from(x)).sum::<u32>(), 3);
    }
}
