//! Задача B: три итератора — по установленным битам, по простым числам
//! (ленивое сегментированное решето) и по кадрам SLIP с байтовым
//! стаффингом.
//!
//! Типы и сигнатуры даны, тела — `todo!()`. Поля `Primes` выберите сами:
//! в памяти должны быть только текущий сегмент и простые до корня из его
//! верхней границы. Для каждого итератора реализуйте `size_hint` честно:
//! нижняя граница не больше, а верхняя не меньше того, что осталось.

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
        todo!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

impl DoubleEndedIterator for Bits {
    fn next_back(&mut self) -> Option<u32> {
        todo!()
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
    // Поля выберите сами.
}

#[cfg(feature = "alloc")]
impl Primes {
    pub fn new() -> Self {
        Self::starting_at(0)
    }

    /// Простые не меньше `from`.
    pub fn starting_at(from: u32) -> Self {
        todo!()
    }

    /// Сколько простых решето держит для просеивания: растёт как число
    /// простых до корня из пройденной границы, а не до корня из 2^32.
    pub fn base_len(&self) -> usize {
        todo!()
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
        todo!()
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
        todo!()
    }
}

impl<'a> Iterator for Slip<'a> {
    type Item = SlipFrame<'a>;

    fn next(&mut self) -> Option<SlipFrame<'a>> {
        todo!()
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
        todo!()
    }

    /// Сколько элементов может остаться: подумайте, что даёт пара байтов
    /// с ошибкой.
    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

impl FusedIterator for Unescape<'_> {}
// ANCHOR_END: slip
