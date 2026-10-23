//! Раздел 13.1 и введение: трейт `Iterator`, `IntoIterator` и три формы
//! `for`, итератор среза изнутри, `by_ref`, диапазоны.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

// ANCHOR: intro
/// Суммы по скользящему окну ширины `w`: их `x.len() - w + 1`, а если окно
/// шире среза, то ни одной.
pub fn window_sums(x: &[i16], w: usize) -> impl Iterator<Item = i32> {
    x.windows(w).map(|win| win.iter().map(|&s| i32::from(s)).sum())
}

/// Индекс последнего отсчёта выше порога.
pub fn last_above(x: &[i16], lim: i16) -> Option<usize> {
    x.iter().rposition(|&s| s > lim)
}

/// Разности соседних отсчётов: их на одну меньше, чем отсчётов.
pub fn deltas(x: &[i16]) -> impl Iterator<Item = i16> {
    x.windows(2).map(|w| w[1] - w[0])
}

/// Убирает выбросы на месте.
#[cfg(feature = "alloc")]
pub fn drop_spikes(x: &mut Vec<i16>, lim: i16) {
    x.retain(|s| (-lim..=lim).contains(s));
}
// ANCHOR_END: intro

/// Ошибка `drop_spikes` из C-примера в Rust не компилируется: пока цикл
/// держит итератор, то есть заём `v`, менять `v` нельзя.
///
/// ```compile_fail,E0502
/// let mut v = vec![10i16, 900, 950, 11];
/// for (i, s) in v.iter().enumerate() {
///     if *s > 500 {
///         v.remove(i);
///     }
/// }
/// ```
pub const fn no_remove_while_iterating() {}

// ANCHOR: countdown
/// Обратный отсчёт: n, n − 1, …, 1.
pub struct Countdown(pub u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let n = self.0;
        if n == 0 {
            return None;
        }
        self.0 = n - 1;
        Some(n)
    }
}
// ANCHOR_END: countdown

// ANCHOR: free_methods
/// Один метод `next` — и все остальные методы трейта достаются даром.
pub fn free_methods() -> (u32, u32, Option<usize>, usize) {
    let odd: u32 = Countdown(5).filter(|n| n % 2 == 1).sum(); // 5 + 3 + 1
    let max = Countdown(4).map(|n| n * n).max().unwrap(); //     16
    let pos = Countdown(10).position(|n| n == 7); //              Some(3)
    let pairs = Countdown(4).zip(Countdown(2)).count(); //       2
    (odd, max, pos, pairs)
}
// ANCHOR_END: free_methods

// ANCHOR: three_for
/// Три формы `for` по одному вектору: по общей ссылке, по изменяемой и по
/// значению.
#[cfg(feature = "alloc")]
pub fn three_forms() -> (u32, Vec<u32>, u32) {
    let mut v = alloc::vec![1u32, 2, 3];
    let mut sum = 0;
    for x in &v {
        // x: &u32, v только читается
        sum += x;
    }
    for x in &mut v {
        // x: &mut u32
        *x *= 10;
    }
    let seen = v.clone();
    let mut total = 0;
    for x in v {
        // x: u32; вектор перемещён в цикл и освобождён в его конце
        total += x;
    }
    (sum, seen, total)
}
// ANCHOR_END: three_for

/// После `for x in v` вектора у функции нет: `into_iter` забрал его.
///
/// ```compile_fail,E0382
/// let v = vec![1u32, 2, 3];
/// for x in v {
///     let _ = x;
/// }
/// let n = v.len();
/// ```
pub const fn for_moves() {}

// ANCHOR: next
/// Один шаг итератора по срезу: сравнить указатель с концом, сдвинуть на
/// элемент вперёд, отдать прежний. `Option<&u32>` — указатель, у которого
/// `None` — ноль.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_next<'a>(it: &mut core::slice::Iter<'a, u32>) -> Option<&'a u32> {
    it.next()
}
// ANCHOR_END: next

// ANCHOR: by_ref
/// Заголовок — байты до первого нуля, тело — всё, что после него. Цикл
/// берёт итератор взаймы через `by_ref`, и после `break` итератор помнит,
/// где остановился.
pub fn split_at_zero(bytes: &[u8]) -> (&[u8], &[u8]) {
    let mut it = bytes.iter();
    let mut head = 0;
    for &b in it.by_ref() {
        if b == 0 {
            break;
        }
        head += 1;
    }
    (&bytes[..head], it.as_slice())
}
// ANCHOR_END: by_ref

// ANCHOR: ranges
/// Диапазоны — сами итераторы. `0..=255u8` выдаёт все 256 значений и
/// останавливается: в `RangeInclusive` есть флаг «исчерпан», и шага за
/// 255 не бывает.
pub fn ranges() -> (usize, u32, [u32; 4]) {
    let n = (0..=255u8).count();
    let sum = (0..=255u8).map(u32::from).sum();
    let mut down = [0; 4];
    for (d, i) in down.iter_mut().zip((0..10).rev().step_by(3)) {
        *d = i;
    }
    (n, sum, down)
}
// ANCHOR_END: ranges

/// Диапазон — итератор, а не пара чисел, и он не `Copy`: цикл забирает
/// его себе.
///
/// ```compile_fail,E0382
/// let r = 0..3;
/// for i in r {
///     let _ = i;
/// }
/// let n = r.len();
/// ```
///
/// Полуоткрытый диапазон до 256 у `u8` не записать: литерал не влезает в
/// тип, и это ошибка, а не предупреждение.
///
/// ```compile_fail
/// for b in 0..256u8 {
///     let _ = b;
/// }
/// ```
pub const fn range_is_iterator() {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::any::type_name;

    const ADC: [i16; 6] = [10, 12, 900, 950, 11, 13];

    #[test]
    fn intro() {
        assert_eq!(window_sums(&ADC, 3).collect::<Vec<_>>(), [922, 1862, 1861, 974]);
        assert_eq!(window_sums(&ADC[..2], 3).count(), 0);
        assert_eq!(last_above(&ADC, 500), Some(3));
        assert_eq!(last_above(&ADC, 1000), None);
        assert_eq!(deltas(&ADC).collect::<Vec<_>>(), [2, 888, 50, -939, 2]);
        assert_eq!(deltas(&ADC[..1]).count(), 0);
        let mut v = ADC.to_vec();
        drop_spikes(&mut v, 500);
        assert_eq!(v, [10, 12, 11, 13]);
        // Отсчёт i16::MIN не ломает сравнение: abs() здесь переполнился бы.
        let mut v = vec![i16::MIN, 0, i16::MAX];
        drop_spikes(&mut v, 500);
        assert_eq!(v, [0]);
    }

    #[test]
    fn countdown() {
        assert_eq!(Countdown(3).collect::<Vec<_>>(), [3, 2, 1]);
        assert_eq!(Countdown(0).next(), None);
        assert_eq!(free_methods(), (9, 16, Some(3), 2));
    }

    #[test]
    fn three_for() {
        assert_eq!(three_forms(), (6, vec![10, 20, 30], 60));
    }

    /// Имена типов, которые выбирает `IntoIterator`, у rustc 1.97.
    #[test]
    fn observed_into_iter_types() {
        assert_eq!(
            type_name::<<&Vec<u32> as IntoIterator>::IntoIter>(),
            "core::slice::iter::Iter<'_, u32>"
        );
        assert_eq!(
            type_name::<<&mut Vec<u32> as IntoIterator>::IntoIter>(),
            "core::slice::iter::IterMut<'_, u32>"
        );
        assert_eq!(
            type_name::<<Vec<u32> as IntoIterator>::IntoIter>(),
            "alloc::vec::into_iter::IntoIter<u32>"
        );
        assert_eq!(
            type_name::<<[u32; 4] as IntoIterator>::IntoIter>(),
            "core::array::iter::IntoIter<u32, 4>"
        );
        assert_eq!(
            type_name::<<core::ops::Range<u8> as IntoIterator>::IntoIter>(),
            "core::ops::range::Range<u8>"
        );
    }

    /// Размеры итераторов у rustc 1.97: раскладка `repr(Rust)` не
    /// гарантирована.
    #[test]
    fn observed_iterator_sizes() {
        let w = size_of::<usize>();
        assert_eq!(size_of::<core::slice::Iter<'_, u32>>(), 2 * w);
        assert_eq!(size_of::<core::ops::Range<usize>>(), 2 * w);
        assert_eq!(size_of::<core::ops::RangeInclusive<u8>>(), 3);
        assert_eq!(size_of::<alloc::vec::IntoIter<u32>>(), 4 * w);
        assert_eq!(size_of::<core::array::IntoIter<u32, 4>>(), 16 + 2 * w);
    }

    #[test]
    fn slice_next() {
        let v = [7u32, 8];
        let mut it = v.iter();
        assert_eq!(ch13_next(&mut it), Some(&7));
        assert_eq!(ch13_next(&mut it), Some(&8));
        assert_eq!(ch13_next(&mut it), None);
        assert_eq!(size_of::<Option<&u32>>(), size_of::<&u32>());
    }

    #[test]
    fn by_ref() {
        assert_eq!(split_at_zero(b"T01\0abc"), (&b"T01"[..], &b"abc"[..]));
        assert_eq!(split_at_zero(b"abc"), (&b"abc"[..], &b""[..]));
        assert_eq!(split_at_zero(b"\0"), (&b""[..], &b""[..]));
    }

    #[test]
    fn range_iterators() {
        assert_eq!(ranges(), (256, 32640, [9, 6, 3, 0]));
    }
}
