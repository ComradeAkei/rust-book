// Раздел 5.6: граница `for<'a>` на GAT подразумевает `'static`.
// Файл не входит в крейт: тест tests/rejected.rs собирает его rustc и
// ждёт ошибку E0521 с пояснением компилятора. Тот же трейт, что в
// src/gat.rs, но в одном файле: пояснение компилятор печатает, только
// когда граница записана в том же крейте.

pub trait Lender {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}

pub struct Normalize<'s> {
    lines: core::str::Lines<'s>,
    buf: String,
}

impl<'s> Normalize<'s> {
    pub fn new(src: &'s str) -> Self {
        Normalize { lines: src.lines(), buf: String::new() }
    }
}

impl<'s> Lender for Normalize<'s> {
    type Item<'a>
        = &'a str
    where
        Self: 'a;

    fn next(&mut self) -> Option<&str> {
        let line = self.lines.next()?;
        self.buf.clear();
        self.buf.extend(line.chars().filter(|c| !c.is_whitespace()));
        Some(&self.buf)
    }
}

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

pub fn bauds(text: &str) -> usize {
    count_keys_hrtb(Normalize::new(text), "baud")
}
