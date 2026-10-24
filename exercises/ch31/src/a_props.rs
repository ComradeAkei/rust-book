//! Задача 31.A: свойства и стратегии proptest для шести функций.
//!
//! Функции даны и верны — их не меняйте. Для каждой нужны стратегия — какие входы
//! порождать — и свойство — что должно выполняться на любом из них.
//! Общие тесты прогоняют пару «стратегия + свойство» с одним и тем же
//! зерном против верной функции и против её мутантов с правдоподобными
//! ошибками: верная должна проходить, каждый мутант — падать. Мутант,
//! которого случайные входы «по умолчанию» не задевают, требует
//! стратегии, которая целится в границы.

use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;

// ------------------------------------------------------------------ 1
// ANCHOR: sum
/// Сумма без переполнения: `None`, если сумма не помещается в `u32`.
pub fn checked_sum(xs: &[u32]) -> Option<u32> {
    xs.iter().try_fold(0u32, |acc, &x| acc.checked_add(x))
}
// ANCHOR_END: sum

/// Входы для [`checked_sum`].
pub fn sum_inputs() -> BoxedStrategy<Vec<u32>> {
    todo!("31.A1: стратегия для checked_sum")
}

/// Свойство [`checked_sum`]: `f` — проверяемая реализация.
pub fn sum_prop(f: fn(&[u32]) -> Option<u32>, xs: &[u32]) -> TestCaseResult {
    let _ = (f, xs);
    todo!("31.A1: свойство checked_sum")
}

// ------------------------------------------------------------------ 2
// ANCHOR: dedup
/// Отсортированный срез без повторов: из каждой серии равных — одно.
pub fn dedup_sorted(xs: &[i32]) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::with_capacity(xs.len());
    for &x in xs {
        if out.last() != Some(&x) {
            out.push(x);
        }
    }
    out
}
// ANCHOR_END: dedup

/// Входы для [`dedup_sorted`]: по условию функции — отсортированные.
pub fn dedup_inputs() -> BoxedStrategy<Vec<i32>> {
    todo!("31.A2: стратегия для dedup_sorted")
}

/// Свойство [`dedup_sorted`].
pub fn dedup_prop(f: fn(&[i32]) -> Vec<i32>, xs: &[i32]) -> TestCaseResult {
    let _ = (f, xs);
    todo!("31.A2: свойство dedup_sorted")
}

// ------------------------------------------------------------------ 3
// ANCHOR: varint
/// LEB128 без знака: по 7 бит, младшие первыми, старший бит байта —
/// «дальше есть ещё». Возвращает число записанных байт, от 1 до 5.
pub fn varint_encode(mut n: u32, out: &mut [u8; 5]) -> usize {
    let mut i = 0;
    loop {
        let low = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            out[i] = low;
            return i + 1;
        }
        out[i] = low | 0x80;
        i += 1;
    }
}

/// Число и сколько байт оно заняло. Принимает только каноническую запись —
/// ту, что выдаёт `varint_encode`: без лишних нулевых групп в конце и без
/// бит сверх 32.
pub fn varint_decode(bytes: &[u8]) -> Option<(u32, usize)> {
    let mut n = 0u32;
    for (i, &b) in bytes.iter().enumerate().take(5) {
        let low = u32::from(b & 0x7F);
        if i == 4 && low > 0x0F {
            return None;
        }
        n |= low << (7 * i);
        if b & 0x80 == 0 {
            return (i == 0 || low != 0).then_some((n, i + 1));
        }
    }
    None
}
// ANCHOR_END: varint

/// Входы для пары [`varint_encode`] и [`varint_decode`]: число для
/// кодера и байты для декодера.
pub fn varint_inputs() -> BoxedStrategy<(u32, Vec<u8>)> {
    todo!("31.A3: стратегия для varint")
}

/// Свойство пары: `enc` и `dec` — проверяемые реализации.
pub fn varint_prop(
    enc: fn(u32, &mut [u8; 5]) -> usize,
    dec: fn(&[u8]) -> Option<(u32, usize)>,
    n: u32,
    bytes: &[u8],
) -> TestCaseResult {
    let _ = (enc, dec, n, bytes);
    todo!("31.A3: свойство varint")
}

// ------------------------------------------------------------------ 4
// ANCHOR: parse_u16
/// Десятичное число от 0 до 65535: только цифры, хотя бы одна, ведущие
/// нули можно, знака и пробелов нельзя.
pub fn parse_u16(s: &str) -> Option<u16> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.bytes().try_fold(0u16, |n, b| n.checked_mul(10)?.checked_add(u16::from(b - b'0')))
}
// ANCHOR_END: parse_u16

/// Входы для [`parse_u16`].
pub fn u16_inputs() -> BoxedStrategy<String> {
    todo!("31.A4: стратегия для parse_u16")
}

/// Свойство [`parse_u16`].
pub fn u16_prop(f: fn(&str) -> Option<u16>, s: &str) -> TestCaseResult {
    let _ = (f, s);
    todo!("31.A4: свойство parse_u16")
}

// ------------------------------------------------------------------ 5
// ANCHOR: align
/// Наименьший адрес не меньше `addr`, кратный `align`. `None`, если
/// `align` не степень двойки или результат не помещается в `usize`.
pub fn align_up(addr: usize, align: usize) -> Option<usize> {
    if !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    Some(addr.checked_add(mask)? & !mask)
}
// ANCHOR_END: align

/// Входы для [`align_up`]: адрес и выравнивание.
pub fn align_inputs() -> BoxedStrategy<(usize, usize)> {
    todo!("31.A5: стратегия для align_up")
}

/// Свойство [`align_up`].
pub fn align_prop(
    f: fn(usize, usize) -> Option<usize>,
    addr: usize,
    align: usize,
) -> TestCaseResult {
    let _ = (f, addr, align);
    todo!("31.A5: свойство align_up")
}

// ------------------------------------------------------------------ 6
// ANCHOR: bits
/// Множество чисел 0..=255 в четырёх словах по 64 бита.
pub trait Bits {
    /// `true`, если числа в множестве не было.
    fn insert(&mut self, i: u8) -> bool;
    /// `true`, если число было в множестве.
    fn remove(&mut self, i: u8) -> bool;
    fn contains(&self, i: u8) -> bool;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Default)]
pub struct BitSet([u64; 4]);

impl Bits for BitSet {
    fn insert(&mut self, i: u8) -> bool {
        let (w, m) = (usize::from(i / 64), 1u64 << (i % 64));
        let new = self.0[w] & m == 0;
        self.0[w] |= m;
        new
    }

    fn remove(&mut self, i: u8) -> bool {
        let (w, m) = (usize::from(i / 64), 1u64 << (i % 64));
        let had = self.0[w] & m != 0;
        self.0[w] &= !m;
        had
    }

    fn contains(&self, i: u8) -> bool {
        self.0[usize::from(i / 64)] & (1 << (i % 64)) != 0
    }

    fn len(&self) -> usize {
        self.0.iter().map(|w| w.count_ones() as usize).sum()
    }
}

/// Операция над множеством для свойства с моделью.
#[derive(Clone, Copy, Debug)]
pub enum BitOp {
    Insert(u8),
    Remove(u8),
    Contains(u8),
    Len,
}
// ANCHOR_END: bits

/// Последовательности операций над [`BitSet`].
pub fn bits_inputs() -> BoxedStrategy<Vec<BitOp>> {
    todo!("31.A6: стратегия для BitSet")
}

/// Свойство: `set` — проверяемая реализация [`Bits`], изначально пустая.
pub fn bits_prop(set: &mut dyn Bits, ops: &[BitOp]) -> TestCaseResult {
    let _ = (set, ops);
    todo!("31.A6: свойство BitSet")
}
