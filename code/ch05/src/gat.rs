//! Раздел 5.6: обобщённые ассоциированные типы и одалживающие итераторы.
//!
//! `Iterator::Item` — один тип на весь итератор, и в нём нельзя сослаться
//! на время жизни заимствования `&mut self` в `next`. Строку из
//! собственного буфера такой итератор вернуть не может:
//!
//! ```compile_fail
//! pub struct Normalize<'s> {
//!     lines: core::str::Lines<'s>,
//!     buf: String,
//! }
//!
//! impl<'s> Iterator for Normalize<'s> {
//!     type Item = &'s str;
//!
//!     fn next(&mut self) -> Option<&'s str> {
//!         let line = self.lines.next()?;
//!         self.buf.clear();
//!         self.buf.extend(line.chars().filter(|c| !c.is_whitespace()));
//!         Some(&self.buf)
//!     }
//! }
//! ```
//!
//! Граница `for<'a>` на GAT подразумевает `'static`, и функция, которая
//! работает со строковым литералом, отвергает буфер из кучи:
//!
//! ```compile_fail,E0521
//! use ch05::gat::{Normalize, count_keys_hrtb};
//!
//! pub fn bauds(text: &str) -> usize {
//!     count_keys_hrtb(Normalize::new(text), "baud")
//! }
//! ```

use alloc::string::String;

// ANCHOR: lender
/// Одалживающий итератор: элемент заимствует сам итератор на время между
/// вызовами `next`. `Item<'a>` — семейство типов, по одному на каждое
/// время жизни заимствования.
pub trait Lender {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}

/// Строки конфигурации без комментариев и пробелов. Каждая строка
/// собирается в один и тот же буфер, без выделения памяти на строку.
pub struct Normalize<'s> {
    lines: core::str::Lines<'s>,
    buf: String,
}

impl<'s> Normalize<'s> {
    pub fn new(src: &'s str) -> Self {
        Normalize { lines: src.lines(), buf: String::new() }
    }

    /// Общая часть обеих реализаций: следующая непустая строка в `buf`.
    fn advance(&mut self) -> Option<&str> {
        loop {
            let line = self.lines.next()?;
            let line = line.split('#').next().unwrap_or("");
            self.buf.clear();
            self.buf.extend(line.chars().filter(|c| !c.is_whitespace()));
            if !self.buf.is_empty() {
                return Some(&self.buf);
            }
        }
    }
}

impl<'s> Lender for Normalize<'s> {
    type Item<'a>
        = &'a str
    where
        Self: 'a;

    fn next(&mut self) -> Option<&str> {
        self.advance()
    }
}
// ANCHOR_END: lender

// ANCHOR: use
/// Сколько непустых строк. Цикл `while let` вместо `for`: `for` работает
/// только с `Iterator`. Строка должна умереть до следующего `next`, и
/// компилятор это проверяет.
pub fn count_lines<L: Lender>(mut l: L) -> usize {
    let mut n = 0;
    while let Some(_line) = l.next() {
        n += 1;
    }
    n
}
// ANCHOR_END: use

// ANCHOR: trap
/// Естественная запись: «для любого `'a` элемент можно прочитать как
/// строку». Компилируется, работает со строковыми литералами — и
/// подразумевает `L: 'static`, потому что `'a` пробегает и `'static`, а
/// `Item<'static>` требует `Self: 'static`.
pub fn count_keys_hrtb<L>(mut l: L, key: &str) -> usize
where
    L: Lender,
    for<'a> L::Item<'a>: AsRef<str>,
{
    let mut n = 0;
    while let Some(line) = l.next() {
        if line.as_ref().split_once('=').map(|(k, _)| k) == Some(key) {
            n += 1;
        }
    }
    n
}
// ANCHOR_END: trap

// ANCHOR: workaround
/// Обход: тип элемента — в отдельном трейте с параметром `'a`. Параметр
/// по умолчанию `&'a Self` добавляет неявное условие `Self: 'a` там, где
/// трейт упомянут, и `for<'a>` пробегает только допустимые `'a`.
pub trait LendItem<'a, Bound = &'a Self> {
    type Item;
}

pub trait Lend: for<'a> LendItem<'a> {
    fn next(&mut self) -> Option<<Self as LendItem<'_>>::Item>;
}

impl<'a, 's> LendItem<'a> for Normalize<'s> {
    type Item = &'a str;
}

impl<'s> Lend for Normalize<'s> {
    fn next(&mut self) -> Option<&str> {
        self.advance()
    }
}

pub fn count_keys<L>(mut l: L, key: &str) -> usize
where
    L: Lend + for<'a> LendItem<'a, Item = &'a str>,
{
    let mut n = 0;
    while let Some(line) = l.next() {
        if line.split_once('=').map(|(k, _)| k) == Some(key) {
            n += 1;
        }
    }
    n
}
// ANCHOR_END: workaround

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "baud = 9600 # скорость\n\n mode=8N1\nbaud=115200\n# baud=1";

    #[test]
    fn lender() {
        let mut n = Normalize::new(TEXT);
        assert_eq!(Lender::next(&mut n), Some("baud=9600"));
        let addr = Lender::next(&mut n).unwrap().as_ptr();
        assert_eq!(Lender::next(&mut n).unwrap().as_ptr(), addr); // тот же буфер
        assert_eq!(Lender::next(&mut n), None);
    }

    #[test]
    fn count() {
        let owned = String::from(TEXT); // текст в куче, не 'static
        assert_eq!(count_lines(Normalize::new(&owned)), 3);
        assert_eq!(count_keys(Normalize::new(&owned), "baud"), 2);
        assert_eq!(count_keys(Normalize::new(&owned), "mode"), 1);
        // Ловушка работает, пока текст — литерал.
        assert_eq!(count_keys_hrtb(Normalize::new(TEXT), "baud"), 2);
    }
}
