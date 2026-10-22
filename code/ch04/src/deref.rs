//! Раздел 4.4: срезы, авторазыменование и приведения при разыменовании.
//!
//! Приведение разыменовывает через `Deref` сколько нужно раз, но не
//! добавляет к цепочке приведение массива к срезу: `&Box<[u8; 4]>` не
//! становится `&[u8]`.
//!
//! ```compile_fail,E0308
//! fn checksum(d: &[u8]) -> u8 {
//!     d.iter().fold(0, |a, b| a ^ b)
//! }
//!
//! let b: Box<[u8; 4]> = Box::new([1, 2, 3, 4]);
//! checksum(&b);
//! ```
//!
//! `.clone()` на ссылке, когда сам тип не `Clone`, копирует ссылку. Линт
//! `noop_method_call` об этом предупреждает; здесь он сделан ошибкой:
//!
//! ```compile_fail
//! #![deny(noop_method_call)]
//! /// Сырые отсчёты АЦП: копировать их незачем, `Clone` нет.
//! pub struct Raw {
//!     pub samples: [u16; 64],
//! }
//!
//! pub fn average(r: &Raw) -> u32 {
//!     let snapshot = r.clone(); // задумано: копия отсчётов
//!     snapshot.samples.iter().map(|&s| s as u32).sum::<u32>() / 64
//! }
//! ```

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ANCHOR: autoderef
/// Метод находится через любое число ссылок и умных указателей: компилятор
/// сам разыменовывает получатель, пока не найдёт тип с таким методом.
pub fn lengths() -> [usize; 4] {
    let s = String::from("кадр");
    let b: Box<String> = Box::new(s.clone());
    let r: &&String = &&s;
    let v: Vec<u8> = vec![1, 2, 3];
    [s.len(), b.len(), r.len(), v.len()]
}
// ANCHOR_END: autoderef

// ANCHOR: coercion
pub fn checksum(d: &[u8]) -> u8 {
    d.iter().fold(0, |acc, b| acc ^ b)
}

pub fn label(name: &str) -> usize {
    name.len()
}

/// `&Vec<u8>` приводится к `&[u8]`, `&String` к `&str`, `&Box<T>` к `&T`:
/// компилятор вставляет разыменование через `Deref`.
pub fn coercions() -> (u8, u8, usize) {
    let v: Vec<u8> = vec![0x0F, 0xF0];
    let b: Box<[u8; 4]> = Box::new([1, 2, 3, 4]);
    let s = String::from("датчик");
    (checksum(&v), checksum(&*b), label(&s))
}

/// Разыменований в одном приведении может быть несколько:
/// `&&Box<String>` → `&Box<String>` → `&String` → `&str`. Clippy такие
/// лишние уровни справедливо не любит, но здесь они и показываются.
#[allow(clippy::borrowed_box, clippy::needless_borrow)]
pub fn deep() -> usize {
    let b: Box<String> = Box::new(String::from("АЦП"));
    let rb: &Box<String> = &b;
    label(&rb)
}
// ANCHOR_END: coercion

// ANCHOR: method_call
/// Вызов метода — это вызов функции с явным первым аргументом. Ссылку на
/// получатель компилятор тоже берёт сам.
pub fn desugared() -> (usize, usize) {
    let mut v: Vec<u8> = Vec::new();
    v.push(1); // Vec::push(&mut v, 1)
    Vec::push(&mut v, 2);
    (v.len(), <[u8]>::len(&v)) // len объявлена у среза, Vec её получает через Deref
}
// ANCHOR_END: method_call

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deref() {
        assert_eq!(lengths(), [8, 8, 8, 3]); // «кадр» — 8 байт UTF-8
        assert_eq!(coercions(), (0xFF, 4, 12));
        assert_eq!(deep(), 6);
        assert_eq!(desugared(), (2, 2));
    }
}
