//! Задача 31.A: свойства и стратегии proptest для шести функций.
//!
//! Функции даны и верны. Для каждой нужны стратегия — какие входы
//! порождать — и свойство — что должно выполняться на любом из них.
//! Общие тесты прогоняют пару «стратегия + свойство» с одним и тем же
//! зерном против верной функции и против её мутантов с правдоподобными
//! ошибками: верная должна проходить, каждый мутант — падать. Мутант,
//! которого случайные входы «по умолчанию» не задевают, требует
//! стратегии, которая целится в границы.

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;
use std::collections::BTreeSet;

// ------------------------------------------------------------------ 1
// ANCHOR: sum
/// Сумма без переполнения: `None`, если сумма не помещается в `u32`.
pub fn checked_sum(xs: &[u32]) -> Option<u32> {
    xs.iter().try_fold(0u32, |acc, &x| acc.checked_add(x))
}
// ANCHOR_END: sum

// ANCHOR: sum_solution
/// Случайные числа почти всегда переполняют сумму уже на втором
/// слагаемом, а сумма ровно на границе не выпадает никогда. Поэтому
/// входы поровну трёх видов: небольшие числа, произвольные и набор,
/// последний элемент которого подогнан так, чтобы сумма оказалась на 2
/// меньше `u32::MAX`, на ней или чуть выше.
pub fn sum_inputs() -> BoxedStrategy<Vec<u32>> {
    let near_max = (vec(0u32..1 << 20, 0..8), -2i64..=2).prop_map(|(mut xs, delta)| {
        let sum: i64 = xs.iter().map(|&x| i64::from(x)).sum();
        let last = (i64::from(u32::MAX) - sum + delta).clamp(0, i64::from(u32::MAX));
        xs.push(last as u32);
        xs
    });
    prop_oneof![vec(0u32..1000, 0..8), vec(any::<u32>(), 0..4), near_max].boxed()
}

/// Модель — та же сумма в `u64`, где переполнения нет.
pub fn sum_prop(f: fn(&[u32]) -> Option<u32>, xs: &[u32]) -> TestCaseResult {
    let wide: u64 = xs.iter().map(|&x| u64::from(x)).sum();
    prop_assert_eq!(f(xs), u32::try_from(wide).ok());
    Ok(())
}
// ANCHOR_END: sum_solution

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

// ANCHOR: dedup_solution
/// У `any::<i32>()` повторов нет, а без повторов функции нечего делать.
/// Значения — из узкого диапазона, чтобы серии были длинными, плюс края
/// типа: мутант со «стражем» `i32::MIN` теряет именно его.
pub fn dedup_inputs() -> BoxedStrategy<Vec<i32>> {
    let value =
        prop_oneof![4 => -3i32..3, 1 => Just(i32::MIN), 1 => Just(i32::MAX), 1 => any::<i32>()];
    vec(value, 0..24)
        .prop_map(|mut xs| {
            xs.sort();
            xs
        })
        .boxed()
}

/// Результат строго возрастает и состоит ровно из значений входа.
pub fn dedup_prop(f: fn(&[i32]) -> Vec<i32>, xs: &[i32]) -> TestCaseResult {
    let out = f(xs);
    prop_assert!(out.windows(2).all(|w| w[0] < w[1]), "{:?} не возрастает строго", out);
    let want: BTreeSet<i32> = xs.iter().copied().collect();
    prop_assert_eq!(out.iter().copied().collect::<BTreeSet<_>>(), want);
    Ok(())
}
// ANCHOR_END: dedup_solution

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

// ANCHOR: varint_solution
/// Числа — с упором на края групп по 7 бит и на 0. Байты — наполовину
/// «по форме записи»: до пяти байт с битом продолжения, последний байт без
/// него (часто 0, 0x0F, 0x10, 0x7F) и мусор после. Случайные байты почти
/// никогда не дают ни `80 00`, ни пятый байт с битами сверх 32.
pub fn varint_inputs() -> BoxedStrategy<(u32, Vec<u8>)> {
    let edge = (0u32..=32, -1i64..=1)
        .prop_map(|(bits, d)| (((1i64 << bits) + d).clamp(0, i64::from(u32::MAX))) as u32);
    let number = prop_oneof![edge, any::<u32>()];
    let last = prop_oneof![Just(0x00u8), Just(0x0F), Just(0x10), Just(0x7F), 0u8..0x80];
    let shaped =
        (vec(any::<u8>(), 0..=5), last, vec(any::<u8>(), 0..2)).prop_map(|(head, last, tail)| {
            let mut bytes: Vec<u8> = head.into_iter().map(|b| b | 0x80).collect();
            bytes.push(last);
            bytes.extend(tail);
            bytes
        });
    let bytes = prop_oneof![shaped, vec(any::<u8>(), 0..7)];
    (number, bytes).boxed()
}

/// Круговой обход для числа, а для байтов — «принятое декодером выдал бы
/// кодер»: тогда запись каноническая, и лишних форм у числа нет.
pub fn varint_prop(
    enc: fn(u32, &mut [u8; 5]) -> usize,
    dec: fn(&[u8]) -> Option<(u32, usize)>,
    n: u32,
    bytes: &[u8],
) -> TestCaseResult {
    let mut out = [0; 5];
    let len = enc(n, &mut out);
    let bits = 32 - n.leading_zeros() as usize;
    prop_assert_eq!(len, bits.div_ceil(7).max(1));
    prop_assert_eq!(dec(&out[..len]), Some((n, len)));
    if let Some((m, used)) = dec(bytes) {
        let mut again = [0; 5];
        let k = enc(m, &mut again);
        prop_assert_eq!(&again[..k], &bytes[..used]);
    }
    Ok(())
}
// ANCHOR_END: varint_solution

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

// ANCHOR: parse_u16_solution
/// Случайная строка почти никогда не число. Нужны числа по обе стороны
/// от 65535, ведущие нули, пустая строка, знаки и пробелы — и немного
/// случайных строк.
pub fn u16_inputs() -> BoxedStrategy<String> {
    let number = prop_oneof![0u32..1000, 65_000u32..66_000, any::<u32>()];
    prop_oneof![
        number.clone().prop_map(|n| n.to_string()),
        (0usize..3, number.clone()).prop_map(|(z, n)| format!("{}{n}", "0".repeat(z))),
        (prop::sample::select(vec!["", "+", "-", " ", "x"]), number)
            .prop_map(|(p, n)| format!("{p}{n}")),
        Just(String::new()),
        "[0-9+ -]{0,7}",
    ]
    .boxed()
}

/// Оракул — `str::parse` из std; знак `+`, который std принимает, по
/// условию запрещён.
pub fn u16_prop(f: fn(&str) -> Option<u16>, s: &str) -> TestCaseResult {
    let want = if s.starts_with('+') { None } else { s.parse::<u16>().ok() };
    prop_assert_eq!(f(s), want, "{:?}", s);
    Ok(())
}
// ANCHOR_END: parse_u16_solution

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

// ANCHOR: align_solution
/// Равномерный адрес почти никогда не выровнен и не лежит у `usize::MAX`;
/// равномерное выравнивание почти никогда не степень двойки. Поэтому
/// адреса — кратные выравниванию, соседние с ними и у верхнего края, а
/// выравнивания — степени двойки и немного произвольных.
pub fn align_inputs() -> BoxedStrategy<(usize, usize)> {
    let align = prop_oneof![
        4 => (0u32..usize::BITS).prop_map(|k| 1usize << k),
        1 => any::<usize>(),
        1 => Just(0usize),
    ];
    (align, any::<usize>(), -2isize..=2, any::<bool>())
        .prop_map(|(align, raw, d, top)| {
            let base = if top { usize::MAX - (raw % 64) } else { raw & !align.wrapping_sub(1) };
            (base.wrapping_add_signed(d), align)
        })
        .boxed()
}

/// Модель — то же в `u128`, где переполнения нет.
pub fn align_prop(
    f: fn(usize, usize) -> Option<usize>,
    addr: usize,
    align: usize,
) -> TestCaseResult {
    let want = align.is_power_of_two().then(|| {
        let a = align as u128;
        (addr as u128).div_ceil(a) * a
    });
    let want = want.and_then(|r| usize::try_from(r).ok());
    prop_assert_eq!(f(addr, align), want, "addr {:#x}, align {:#x}", addr, align);
    Ok(())
}
// ANCHOR_END: align_solution

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

// ANCHOR: bits_solution
/// Числа — по всем четырём словам, с упором на границы слов, и часто
/// одни и те же: иначе удалять почти всегда нечего.
pub fn bits_inputs() -> BoxedStrategy<Vec<BitOp>> {
    let index = prop_oneof![
        Just(0u8),
        Just(63),
        Just(64),
        Just(127),
        Just(128),
        Just(191),
        Just(192),
        Just(255),
        any::<u8>(),
    ];
    let op = prop_oneof![
        3 => index.clone().prop_map(BitOp::Insert),
        2 => index.clone().prop_map(BitOp::Remove),
        1 => index.prop_map(BitOp::Contains),
        1 => Just(BitOp::Len),
    ];
    vec(op, 0..64).boxed()
}

/// После каждой операции множество ведёт себя как `BTreeSet<u8>`: те же
/// ответы и тот же размер.
pub fn bits_prop(set: &mut dyn Bits, ops: &[BitOp]) -> TestCaseResult {
    let mut model = BTreeSet::new();
    for &op in ops {
        match op {
            BitOp::Insert(i) => prop_assert_eq!(set.insert(i), model.insert(i), "{:?}", op),
            BitOp::Remove(i) => prop_assert_eq!(set.remove(i), model.remove(&i), "{:?}", op),
            BitOp::Contains(i) => prop_assert_eq!(set.contains(i), model.contains(&i), "{:?}", op),
            BitOp::Len => {}
        }
        prop_assert_eq!(set.len(), model.len(), "после {:?}", op);
    }
    Ok(())
}
// ANCHOR_END: bits_solution
