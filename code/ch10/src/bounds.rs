//! Раздел 10.2: ограничения, `where`, условные методы и `T: 'static`.
//!
//! Ограничения на `impl` делают методы условными. `to_string` у
//! `Labeled<T>` есть, только если у `T` есть `Display`:
//!
//! ```compile_fail,E0599
//! use ch10::bounds::Labeled;
//! struct Raw(u16);
//! let s = Labeled { name: "АЦП", value: Raw(812) }.to_string();
//! ```
//!
//! Ограничение, записанное в определении типа, приходится повторять в
//! каждом `impl`, иначе E0277:
//!
//! ```compile_fail,E0277
//! pub struct Sorted<T: Ord>(Vec<T>);
//! impl<T> Sorted<T> {
//!     pub fn len(&self) -> usize { self.0.len() }
//! }
//! ```
//!
//! `T: 'static` — свойство типа, а не значения: в `T` нет ссылок короче
//! `'static`. Локальная `String` ему отвечает, ссылка на неё — нет:
//!
//! ```
//! use ch10::bounds::assert_static;
//! let s = String::from("локальная строка");
//! assert_static(&s); // T = String
//! ```
//!
//! ```compile_fail,E0597
//! use ch10::bounds::assert_static;
//! let s = String::from("локальная строка");
//! let r = &s;
//! assert_static(&r); // T = &String с временем жизни s
//! ```

use core::fmt;

// ANCHOR: where
/// Среднее отсчётов любого целого типа, который без потерь расширяется до
/// `i32`: `[i16; N]`, `&[u8]` через `copied()`, итератор `u16`.
pub fn average<I>(samples: I) -> Option<i32>
where
    I: IntoIterator,
    I::Item: Into<i32>,
{
    let mut sum = 0i64;
    let mut n = 0i64;
    for s in samples {
        sum += i64::from(s.into());
        n += 1;
    }
    if n == 0 { None } else { Some((sum / n) as i32) }
}
// ANCHOR_END: where

// ANCHOR: labeled
/// Значение с подписью. Само определение ничего от `T` не требует.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Labeled<T> {
    pub name: &'static str,
    pub value: T,
}

/// `Display` есть у `Labeled<T>`, только если он есть у `T`. С ним
/// приходят `to_string` и `format!("{}")`.
impl<T: fmt::Display> fmt::Display for Labeled<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = {}", self.name, self.value)
    }
}

/// Метод для любых `T`, которые можно сравнивать с порогом того же типа.
impl<T: PartialOrd> Labeled<T> {
    pub fn exceeds(&self, limit: &T) -> bool {
        self.value > *limit
    }
}
// ANCHOR_END: labeled

// ANCHOR: static
/// Принимает ссылку на что угодно, лишь бы тип `T` не содержал ссылок
/// короче `'static`.
pub fn assert_static<T: 'static>(_: &T) {}
// ANCHOR_END: static

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn where_clause() {
        assert_eq!(average([-300i16, 4000, 12, 0]), Some(928));
        assert_eq!(average([1u8, 2, 4].iter().copied()), Some(2));
        assert_eq!(average(vec![60_000u16, 60_000]), Some(60_000));
        assert_eq!(average(core::iter::empty::<i8>()), None);
    }

    #[test]
    fn conditional_methods() {
        let t = Labeled { name: "t", value: 21.5f32 };
        assert_eq!(t.to_string(), "t = 21.5");
        assert!(t.exceeds(&20.0));
        let raw = Labeled { name: "raw", value: [1u8, 2] };
        assert!(raw.exceeds(&[1, 1]));
        assert_static(&raw);
    }
}
