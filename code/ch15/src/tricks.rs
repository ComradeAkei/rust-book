//! Раздел 15.9: трюки и ловушки коллекций.
//!
//! `BTreeMap::range` с началом больше конца — паника, а не пустой ответ:
//!
//! ```should_panic
//! use std::collections::BTreeMap;
//! let m: BTreeMap<u32, i32> = (0..10).map(|t| (t, 0)).collect();
//! let (from, to) = (8, 3);
//! let _ = m.range(from..to).count();
//! ```

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ANCHOR: retain
/// Убрать выбросы на месте: `retain` сдвигает оставшиеся элементы за один
/// проход, без второго вектора и без O(n²) от `remove` в цикле.
pub fn drop_outliers(v: &mut Vec<i32>, lo: i32, hi: i32) {
    v.retain(|&x| (lo..=hi).contains(&x));
}
// ANCHOR_END: retain

// ANCHOR: extract_if
/// Разделить на месте: тревожные значения уходят в отдельный вектор,
/// остальные остаются в `v` в прежнем порядке.
pub fn split_alarms(v: &mut Vec<i32>, limit: i32) -> Vec<i32> {
    v.extract_if(.., |x| *x > limit).collect()
}
// ANCHOR_END: extract_if

// ANCHOR: drain
/// Отдать первые `n` измерений на отправку: `drain` вынимает диапазон и
/// сдвигает хвост один раз, когда итератор уничтожен.
pub fn take_batch(v: &mut Vec<i32>, n: usize) -> Vec<i32> {
    v.drain(..n.min(v.len())).collect()
}
// ANCHOR_END: drain

// ANCHOR: dedup
/// Повторы подряд — частая картина у медленно меняющегося датчика.
/// `dedup_by_key` оставляет первое из серии равных.
pub fn changes(v: &mut Vec<(u32, i32)>) {
    v.dedup_by_key(|&mut (_, value)| value);
}
// ANCHOR_END: dedup

// ANCHOR: sorted_insert
/// Вставка в отсортированный `Vec` с сохранением порядка: двоичный поиск
/// места и сдвиг хвоста. Сдвиг — один `memmove`, на тысячах элементов это
/// быстрее, чем выделение узла дерева.
pub fn insert_sorted(v: &mut Vec<u32>, x: u32) {
    let i = v.partition_point(|&y| y < x);
    v.insert(i, x);
}
// ANCHOR_END: sorted_insert

// ANCHOR: swap_remove
/// Удалить по индексу за O(1), если порядок не важен: на место удалённого
/// встаёт последний.
pub fn forget_sensor(active: &mut Vec<u16>, i: usize) -> u16 {
    active.swap_remove(i)
}
// ANCHOR_END: swap_remove

// ANCHOR: reuse
/// Один буфер на все кадры: `clear` обнуляет длину, ёмкость остаётся, и
/// после первого кадра распределитель больше не нужен.
pub fn encode_all(frames: &[&[u8]], mut sink: impl FnMut(&[u8])) -> usize {
    let mut buf = Vec::new();
    let mut max_cap = 0;
    for f in frames {
        buf.clear();
        buf.extend_from_slice(&[0xA5, f.len() as u8]);
        buf.extend_from_slice(f);
        sink(&buf);
        max_cap = max_cap.max(buf.capacity());
    }
    max_cap
}
// ANCHOR_END: reuse

// ANCHOR: clone_trap
/// `vec![x; n]` клонирует `x` n − 1 раз и кладёт сам `x` последним, а
/// `clone` у `Vec` выделяет ровно `len`: ёмкость остаётся только у
/// последнего.
pub fn rows_with_capacity(n: usize, cap: usize) -> Vec<usize> {
    let rows: Vec<Vec<u8>> = alloc::vec![Vec::with_capacity(cap); n];
    rows.iter().map(Vec::capacity).collect()
}

/// Так ёмкость будет у каждой строки.
pub fn rows_with_capacity_fixed(n: usize, cap: usize) -> Vec<usize> {
    let rows: Vec<Vec<u8>> = (0..n).map(|_| Vec::with_capacity(cap)).collect();
    rows.iter().map(Vec::capacity).collect()
}
// ANCHOR_END: clone_trap

// ANCHOR: chunk_by
/// Серии измерений без разрывов дольше `gap_ms`: `chunk_by` режет срез там,
/// где условие для соседей ложно.
pub fn sessions(times: &[u32], gap_ms: u32) -> Vec<(u32, u32)> {
    times.chunk_by(|a, b| b - a <= gap_ms).map(|run| (run[0], run[run.len() - 1])).collect()
}
// ANCHOR_END: chunk_by

// ANCHOR: btree_entry
/// Минимум и максимум по каждому датчику за один проход: `and_modify`
/// меняет имеющееся значение, `or_insert` вставляет первое.
pub fn min_max(readings: &[(u16, i32)]) -> BTreeMap<u16, (i32, i32)> {
    let mut m = BTreeMap::new();
    for &(s, v) in readings {
        m.entry(s)
            .and_modify(|(lo, hi): &mut (i32, i32)| {
                *lo = (*lo).min(v);
                *hi = (*hi).max(v);
            })
            .or_insert((v, v));
    }
    m
}
// ANCHOR_END: btree_entry

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn retain_extract_drain_dedup() {
        let mut v = vec![5, -900, 7, 30_000, 6];
        drop_outliers(&mut v, -100, 100);
        assert_eq!(v, [5, 7, 6]);
        let mut v = vec![1, 50, 2, 70, 3];
        assert_eq!(split_alarms(&mut v, 10), [50, 70]);
        assert_eq!(v, [1, 2, 3]);
        assert_eq!(take_batch(&mut v, 2), [1, 2]);
        assert_eq!(take_batch(&mut v, 5), [3]);
        let mut s = vec![(0, 1), (10, 1), (20, 2), (30, 2), (40, 1)];
        changes(&mut s);
        assert_eq!(s, [(0, 1), (20, 2), (40, 1)]);
    }

    #[test]
    fn sorted_and_swap() {
        let mut v = vec![10, 20, 30];
        insert_sorted(&mut v, 25);
        insert_sorted(&mut v, 5);
        assert_eq!(v, [5, 10, 20, 25, 30]);
        let mut a = vec![1, 2, 3, 4];
        assert_eq!(forget_sensor(&mut a, 1), 2);
        assert_eq!(a, [1, 4, 3]);
    }

    #[test]
    fn reuse_keeps_capacity() {
        let frames: [&[u8]; 3] = [&[1, 2, 3], &[4], &[5, 6]];
        let mut seen = Vec::new();
        let cap = encode_all(&frames, |b| seen.push(b.len()));
        assert_eq!(seen, [5, 3, 4]);
        assert!(cap >= 5);
    }

    #[test]
    fn clone_drops_capacity() {
        assert_eq!(rows_with_capacity(3, 16), [0, 0, 16]);
        assert_eq!(rows_with_capacity_fixed(3, 16), [16, 16, 16]);
    }

    #[test]
    fn sessions_split_on_gaps() {
        assert_eq!(
            sessions(&[0, 10, 20, 500, 510, 2000], 100),
            [(0, 20), (500, 510), (2000, 2000)]
        );
    }

    #[test]
    fn min_max_per_sensor() {
        let m = min_max(&[(1, 5), (2, 0), (1, -3), (1, 9)]);
        assert_eq!(m[&1], (-3, 9));
        assert_eq!(m[&2], (0, 0));
    }
}
