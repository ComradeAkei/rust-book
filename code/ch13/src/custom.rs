//! Раздел 13.4: собственные итераторы — кольцевой буфер приёма с
//! `DoubleEndedIterator`, `ExactSizeIterator` и `FusedIterator`, контракт
//! `size_hint`, итераторы из замыканий: `from_fn` и `successors`.

use core::iter::{FusedIterator, from_fn, successors};
use core::slice;

// ANCHOR: ring
/// Кольцевой буфер приёма UART на `N` байт: обработчик прерывания кладёт
/// байты в конец, основной цикл забирает из начала.
pub struct Ring<const N: usize> {
    buf: [u8; N],
    start: usize,
    len: usize,
}

impl<const N: usize> Ring<N> {
    pub const fn new() -> Self {
        Ring { buf: [0; N], start: 0, len: 0 }
    }

    /// Кладёт байт; `false`, если буфер полон и байт потерян.
    pub fn push(&mut self, b: u8) -> bool {
        if self.len == N {
            return false;
        }
        self.buf[(self.start + self.len) % N] = b;
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        let b = self.buf[self.start];
        self.start = (self.start + 1) % N;
        self.len -= 1;
        Some(b)
    }

    /// Содержимое двумя срезами: от начала до конца массива и с начала
    /// массива. Второй пуст, если данные не переходят через край.
    pub fn as_slices(&self) -> (&[u8], &[u8]) {
        let end = self.start + self.len;
        if end <= N {
            (&self.buf[self.start..end], &[])
        } else {
            (&self.buf[self.start..], &self.buf[..end - N])
        }
    }

    pub fn iter(&self) -> RingIter<'_> {
        let (a, b) = self.as_slices();
        RingIter { a: a.iter(), b: b.iter() }
    }
}

impl<const N: usize> Default for Ring<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Обход содержимого без копирования: сначала первая половина, потом
/// вторая.
pub struct RingIter<'a> {
    a: slice::Iter<'a, u8>,
    b: slice::Iter<'a, u8>,
}

impl<'a> Iterator for RingIter<'a> {
    type Item = &'a u8;

    fn next(&mut self) -> Option<&'a u8> {
        self.a.next().or_else(|| self.b.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.a.len() + self.b.len();
        (n, Some(n))
    }

    /// Свёртка двумя простыми циклами, без проверки «какая половина» на
    /// каждом шаге. На ней работают `sum`, `count`, `for_each` и другие.
    fn fold<B, F: FnMut(B, &'a u8) -> B>(self, init: B, mut f: F) -> B {
        let acc = self.a.fold(init, &mut f);
        self.b.fold(acc, f)
    }
}

impl DoubleEndedIterator for RingIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.b.next_back().or_else(|| self.a.next_back())
    }
}

/// `len` берётся из `size_hint`, поэтому тело пустое.
impl ExactSizeIterator for RingIter<'_> {}

/// Кончившись, больше не возобновляется: обе половины — срезы.
impl FusedIterator for RingIter<'_> {}

/// `for b in &ring` — так же, как `for x in &v` у вектора.
impl<'a, const N: usize> IntoIterator for &'a Ring<N> {
    type Item = &'a u8;
    type IntoIter = RingIter<'a>;

    fn into_iter(self) -> RingIter<'a> {
        self.iter()
    }
}
// ANCHOR_END: ring

/// Готовый `a.iter().chain(b)` дал бы тот же обход, но не длину: у
/// `Chain` нет `ExactSizeIterator`, ведь сумма двух длин может не
/// поместиться в `usize`.
///
/// ```compile_fail,E0599
/// fn total(a: &[u8], b: &[u8]) -> usize {
///     a.iter().chain(b).len()
/// }
/// ```
pub const fn chain_has_no_len() {}

// ANCHOR: ring_use
/// Всё, что даёт `RingIter`: обход в обе стороны, длина без обхода, поиск
/// с конца, `for` по ссылке на буфер.
pub fn ring_demo() -> (usize, Option<u8>, Option<usize>, u32, [u8; 3]) {
    let mut r = Ring::<8>::new();
    for b in 1..=6 {
        r.push(b);
    }
    for _ in 0..4 {
        r.pop();
    }
    for b in 7..=10 {
        r.push(b); // 9 и 10 ложатся в начало массива
    }
    let len = r.iter().len(); //                      6: 5 6 7 8 9 10
    let last = r.iter().next_back().copied(); //       Some(10)
    let pos = r.iter().rposition(|&b| b % 4 == 0); //  Some(3): байт 8
    let mut sum = 0;
    for b in &r {
        sum += u32::from(*b);
    }
    let mut tail = [0; 3];
    for (t, b) in tail.iter_mut().zip(r.iter().rev()) {
        *t = *b;
    }
    (len, last, pos, sum, tail)
}
// ANCHOR_END: ring_use

// ANCHOR: size_hint
/// Итератор, который врёт в `size_hint`. Безопасный код не вправе
/// полагаться на подсказку, и `collect` с ней справляется; полагаться на
/// неё вправе только `unsafe`-код через трейт `TrustedLen`, который пока
/// нестабилен.
pub struct Liar {
    left: u8,
    claim: (usize, Option<usize>),
}

impl Liar {
    pub const fn new(left: u8, claim: (usize, Option<usize>)) -> Self {
        Liar { left, claim }
    }
}

impl Iterator for Liar {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.left = self.left.checked_sub(1)?;
        Some(self.left)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.claim
    }
}
// ANCHOR_END: size_hint

// ANCHOR: from_fn
/// Опрос регистра данных UART: `None` значит «FIFO пуст сейчас», а не
/// «байтов больше не будет». Модель регистра — заранее записанная
/// последовательность ответов.
pub fn poll<'a>(answers: &'a [Option<u8>]) -> impl Iterator<Item = u8> + 'a {
    let mut i = 0;
    from_fn(move || {
        let r = answers.get(i).copied().flatten();
        i += 1;
        r
    })
}

/// `for` останавливается на первом `None`, а итератор после него
/// продолжает отдавать байты. `fuse` обрывает его навсегда.
pub fn poll_demo() -> (usize, Option<u8>, Option<u8>) {
    let answers = [Some(1), Some(2), None, Some(3), None];
    let mut rx = poll(&answers);
    let mut got = 0;
    for _ in rx.by_ref() {
        got += 1; //                    1 и 2, затем None: цикл окончен
    }
    let later = rx.next(); //           Some(3): пришёл новый байт
    let mut fused = poll(&answers).fuse();
    for _ in fused.by_ref() {}
    (got, later, fused.next()) //       None: fuse запомнил первый None
}
// ANCHOR_END: from_fn

// ANCHOR: successors
/// Степени десяти, которые помещаются в `u32`: каждая получается из
/// предыдущей, и `None` от `checked_mul` заканчивает последовательность.
pub fn powers_of_ten() -> impl Iterator<Item = u32> {
    successors(Some(1u32), |&x| x.checked_mul(10))
}

/// Число десятичных цифр: сколько раз можно делить на 10.
pub fn digits(n: u32) -> usize {
    successors(Some(n), |&n| (n >= 10).then_some(n / 10)).count()
}

/// PRBS-7 (x⁷ + x⁶ + 1), псевдослучайная последовательность для проверки
/// линий связи: регистр сдвига из семи бит, бит обратной связи — XOR
/// старших двух. Последовательность бесконечна, длину задаёт `take`.
pub fn prbs7(seed: u8) -> impl Iterator<Item = u8> {
    successors(Some(seed & 0x7f), |&s| {
        let bit = ((s >> 6) ^ (s >> 5)) & 1;
        Some(((s << 1) | bit) & 0x7f)
    })
}
// ANCHOR_END: successors

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn ring() {
        assert_eq!(ring_demo(), (6, Some(10), Some(3), 45, [10, 9, 8]));
        let mut r = Ring::<2>::new();
        assert!(r.push(1) && r.push(2) && !r.push(3));
        assert_eq!(r.iter().copied().collect::<Vec<_>>(), [1, 2]);
        assert_eq!((r.pop(), r.pop(), r.pop()), (Some(1), Some(2), None));
        assert_eq!(r.iter().len(), 0);
        // Обход совпадает с pop при любом сдвиге начала.
        for shift in 0..8 {
            let mut r = Ring::<8>::new();
            for _ in 0..shift {
                r.push(0);
                r.pop();
            }
            for b in 0..7 {
                r.push(b);
            }
            let it: Vec<u8> = r.iter().copied().collect();
            let back: Vec<u8> = r.iter().rev().copied().collect();
            let sum: u32 = r.iter().map(|&b| u32::from(b)).sum();
            let mut popped = Vec::new();
            while let Some(b) = r.pop() {
                popped.push(b);
            }
            assert_eq!(it, popped);
            assert_eq!(back.iter().rev().copied().collect::<Vec<_>>(), popped);
            assert_eq!(sum, 21);
        }
    }

    #[test]
    fn ring_iter_is_fused() {
        let mut r = Ring::<4>::new();
        r.push(1);
        let mut it = r.iter();
        assert_eq!((it.next(), it.next(), it.next()), (Some(&1), None, None));
    }

    #[test]
    fn lying_size_hint() {
        let v: Vec<u8> = Liar::new(3, (100, Some(100))).collect();
        assert_eq!(v, [2, 1, 0]);
        assert!(v.capacity() >= 100);
        let v: Vec<u8> = Liar::new(3, (0, Some(0))).collect();
        assert_eq!(v, [2, 1, 0]);
    }

    #[test]
    fn std_hints() {
        let x = [1, 2, 3, 4];
        assert_eq!(x.iter().filter(|&&v| v > 2).size_hint(), (0, Some(4)));
        assert_eq!(x.iter().chain(&x).size_hint(), (8, Some(8)));
        assert_eq!(x.iter().flat_map(|&v| 0..v).size_hint(), (0, None));
        assert_eq!(x.iter().skip(std::hint::black_box(10)).size_hint(), (0, Some(0)));
    }

    #[test]
    fn from_fn_poll() {
        assert_eq!(poll_demo(), (2, Some(3), None));
    }

    #[test]
    fn successors_examples() {
        assert_eq!(powers_of_ten().count(), 10);
        assert_eq!(powers_of_ten().last(), Some(1_000_000_000));
        assert_eq!([0, 9, 10, 99, 100, u32::MAX].map(digits), [1, 1, 2, 2, 3, 10]);
        // Период PRBS-7 — 127, и за период встречаются все ненулевые
        // состояния регистра.
        assert_eq!(prbs7(1).skip(1).position(|s| s == 1), Some(126));
        let states: BTreeSet<u8> = prbs7(0x5a).take(127).collect();
        assert_eq!(states.len(), 127);
        assert!(!states.contains(&0));
        // Нулевое состояние — неподвижная точка: регистр застревает.
        assert!(prbs7(0).take(5).all(|s| s == 0));
    }
}
