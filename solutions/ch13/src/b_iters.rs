//! Задача B: три итератора — по установленным битам, по простым числам
//! (ленивое сегментированное решето) и по кадрам SLIP с байтовым
//! стаффингом.

use core::iter::FusedIterator;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

// ANCHOR: bits
/// Номера установленных битов слова: от младшего к старшему, а с конца —
/// от старшего. Так перебирают, например, регистр ожидающих прерываний.
#[derive(Clone, Copy, Debug)]
pub struct Bits(pub u32);

impl Iterator for Bits {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        let i = self.0.trailing_zeros();
        self.0 &= self.0 - 1; // сбросить младший установленный бит
        Some(i)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.count_ones() as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Bits {
    fn next_back(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        let i = 31 - self.0.leading_zeros();
        self.0 &= !(1 << i);
        Some(i)
    }
}

impl ExactSizeIterator for Bits {}

impl FusedIterator for Bits {}
// ANCHOR_END: bits

// ANCHOR: primes
/// Сколько нечётных чисел проверяет один сегмент решета.
#[cfg(feature = "alloc")]
pub const SEGMENT: usize = 8192;

/// Простые числа не меньше `from` по возрастанию, вплоть до последнего
/// простого, которое помещается в `u32`, — 4294967291. Решето идёт
/// сегментами по [`SEGMENT`] нечётных чисел, и в памяти лежат только
/// текущий сегмент и простые до корня из его верхней границы.
#[cfg(feature = "alloc")]
pub struct Primes {
    /// Первое нечётное число сегмента. В `u64`, чтобы граница сегмента у
    /// 2^32 не переполнилась.
    lo: u64,
    /// `composite[i]` — число `lo + 2i` составное.
    composite: [bool; SEGMENT],
    /// Следующий непроверенный индекс в сегменте.
    idx: usize,
    /// Нечётные простые от 3 до `base_limit` по возрастанию.
    base: Vec<u32>,
    base_limit: u32,
    /// Двойку ещё надо выдать.
    two: bool,
}

#[cfg(feature = "alloc")]
impl Primes {
    pub fn new() -> Self {
        Self::starting_at(0)
    }

    pub fn starting_at(from: u32) -> Self {
        let first_odd = u64::from(from.max(3) | 1);
        let mut p = Primes {
            lo: first_odd,
            composite: [false; SEGMENT],
            idx: 0,
            base: Vec::new(),
            base_limit: 1,
            two: from <= 2,
        };
        p.sieve();
        p
    }

    /// Сколько простых решето держит для просеивания: растёт как число
    /// простых до корня из пройденной границы, а не до корня из 2^32.
    pub fn base_len(&self) -> usize {
        self.base.len()
    }

    /// Дополнить `base` нечётными простыми до `limit` пробным делением на
    /// уже найденные: `limit` не больше 65536, и это дёшево.
    fn grow_base(&mut self, limit: u32) {
        let mut n = self.base_limit + 1 + self.base_limit % 2;
        while n <= limit {
            if self.base.iter().take_while(|&&p| p * p <= n).all(|&p| n % p != 0) {
                self.base.push(n);
            }
            n += 2;
        }
        self.base_limit = self.base_limit.max(limit);
    }

    /// Просеять сегмент `[lo, lo + 2·SEGMENT)`.
    fn sieve(&mut self) {
        let hi = self.lo + 2 * SEGMENT as u64;
        self.grow_base(hi.isqrt() as u32);
        self.composite = [false; SEGMENT];
        for &p in &self.base {
            let p = u64::from(p);
            // Первое нечётное кратное p, не меньше lo и не меньше p².
            let mut m = self.lo.div_ceil(p) * p;
            if m % 2 == 0 {
                m += p;
            }
            let mut j = (m.max(p * p) - self.lo) / 2;
            while j < SEGMENT as u64 {
                self.composite[j as usize] = true;
                j += p;
            }
        }
        self.idx = 0;
    }
}

#[cfg(feature = "alloc")]
impl Default for Primes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "alloc")]
impl Iterator for Primes {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.two {
            self.two = false;
            return Some(2);
        }
        loop {
            while self.idx < SEGMENT {
                let n = self.lo + 2 * self.idx as u64;
                if n > u64::from(u32::MAX) {
                    self.idx = SEGMENT;
                    return None;
                }
                self.idx += 1;
                if !self.composite[self.idx - 1] {
                    return Some(n as u32);
                }
            }
            if self.lo + 2 * SEGMENT as u64 > u64::from(u32::MAX) {
                return None;
            }
            self.lo += 2 * SEGMENT as u64;
            self.sieve();
        }
    }
}

#[cfg(feature = "alloc")]
impl FusedIterator for Primes {}
// ANCHOR_END: primes

// ANCHOR: slip
pub const END: u8 = 0xC0;
pub const ESC: u8 = 0xDB;
pub const ESC_END: u8 = 0xDC;
pub const ESC_ESC: u8 = 0xDD;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlipError {
    /// После ESC не ESC_END и не ESC_ESC.
    BadEscape(u8),
    /// ESC последним байтом кадра.
    TrailingEscape,
}

/// Кадры SLIP (RFC 1055) в потоке байт: всё, что между двумя END. Пустые
/// кадры — END подряд, ими отправитель сбрасывает шум линии, —
/// пропускаются. Хвост без завершающего END остаётся в [`Slip::rest`].
#[derive(Clone, Debug)]
pub struct Slip<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Slip<'a> {
    pub const fn new(buf: &'a [u8]) -> Self {
        Slip { buf, pos: 0 }
    }

    /// Байты после последнего END: начало кадра, который ещё не пришёл.
    pub fn rest(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }
}

impl<'a> Iterator for Slip<'a> {
    type Item = SlipFrame<'a>;

    fn next(&mut self) -> Option<SlipFrame<'a>> {
        loop {
            let tail = &self.buf[self.pos..];
            let end = tail.iter().position(|&b| b == END)?;
            self.pos += end + 1;
            if end > 0 {
                return Some(SlipFrame(&tail[..end]));
            }
        }
    }
}

impl FusedIterator for Slip<'_> {}

/// Кадр в том виде, как он пришёл, — с экранированием.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SlipFrame<'a>(pub &'a [u8]);

impl<'a> SlipFrame<'a> {
    /// Байты кадра без экранирования, по одному и без копирования.
    pub fn bytes(&self) -> Unescape<'a> {
        Unescape { it: self.0.iter(), failed: false }
    }
}

/// Снимает экранирование. После первой ошибки кадр испорчен, и итератор
/// заканчивается.
#[derive(Clone, Debug)]
pub struct Unescape<'a> {
    it: core::slice::Iter<'a, u8>,
    failed: bool,
}

impl Iterator for Unescape<'_> {
    type Item = Result<u8, SlipError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        let r = match *self.it.next()? {
            ESC => match self.it.next() {
                Some(&ESC_END) => Ok(END),
                Some(&ESC_ESC) => Ok(ESC),
                Some(&b) => Err(SlipError::BadEscape(b)),
                None => Err(SlipError::TrailingEscape),
            },
            b => Ok(b),
        };
        self.failed = r.is_err();
        Some(r)
    }

    /// Каждый элемент забирает один или два байта. Снизу — один элемент:
    /// первая же пара может оказаться ошибкой, и на ней итератор кончится.
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.failed {
            return (0, Some(0));
        }
        let n = self.it.len();
        (usize::from(n > 0), Some(n))
    }
}

impl FusedIterator for Unescape<'_> {}
// ANCHOR_END: slip
