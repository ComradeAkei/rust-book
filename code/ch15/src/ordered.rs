//! Раздел 15.4: упорядоченные коллекции и очереди. `BTreeMap` для
//! запросов по диапазону, отсортированный `Vec` с двоичным поиском,
//! `VecDeque` как кольцевой буфер последних измерений и `BinaryHeap` как
//! очередь таймеров.

use alloc::collections::{BTreeMap, BinaryHeap, VecDeque};
use alloc::vec::Vec;
use core::cmp::Reverse;

// ANCHOR: btree_range
/// Измерения одного датчика по времени: `range` отдаёт отрезок в порядке
/// ключей за O(log n) на поиск начала.
pub fn between(log: &BTreeMap<u32, i32>, from_ms: u32, to_ms: u32) -> Vec<(u32, i32)> {
    log.range(from_ms..to_ms).map(|(&t, &v)| (t, v)).collect()
}
// ANCHOR_END: btree_range

// ANCHOR: sorted_vec
/// То же на отсортированном `Vec<(u32, i32)>`: `partition_point` — двоичный
/// поиск первой точки, для которой условие ложно.
pub fn between_sorted(log: &[(u32, i32)], from_ms: u32, to_ms: u32) -> &[(u32, i32)] {
    let a = log.partition_point(|&(t, _)| t < from_ms);
    let b = log.partition_point(|&(t, _)| t < to_ms);
    &log[a..b.max(a)]
}
// ANCHOR_END: sorted_vec

// ANCHOR: window
/// Последние `N` измерений и их сумма: кольцевой буфер на `VecDeque`.
/// Старое значение уходит с головы, новое приходит в хвост, буфер не
/// растёт после первых `N`.
pub struct Window {
    buf: VecDeque<i32>,
    sum: i64,
    n: usize,
}

impl Window {
    pub fn new(n: usize) -> Self {
        assert!(n > 0);
        Window { buf: VecDeque::with_capacity(n), sum: 0, n }
    }

    pub fn push(&mut self, v: i32) {
        if self.buf.len() == self.n {
            self.sum -= self.buf.pop_front().unwrap() as i64;
        }
        self.buf.push_back(v);
        self.sum += v as i64;
    }

    /// Среднее, округлённое к нулю.
    pub fn mean(&self) -> Option<i32> {
        (!self.buf.is_empty()).then(|| (self.sum / self.buf.len() as i64) as i32)
    }

    /// Две части кольца: от головы до конца буфера и от начала буфера.
    pub fn as_slices(&self) -> (&[i32], &[i32]) {
        self.buf.as_slices()
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }
}
// ANCHOR_END: window

// ANCHOR: timers
/// Таймеры опроса датчиков: ближайший срок — на вершине. `BinaryHeap` —
/// куча по максимуму, `Reverse` превращает её в кучу по минимуму.
#[derive(Default)]
pub struct Timers {
    heap: BinaryHeap<Reverse<(u32, u16)>>,
}

impl Timers {
    pub fn at(&mut self, deadline_ms: u32, sensor: u16) {
        self.heap.push(Reverse((deadline_ms, sensor)));
    }

    /// Датчики, срок которых наступил к `now_ms`, по возрастанию срока.
    pub fn due(&mut self, now_ms: u32) -> Vec<u16> {
        let mut out = Vec::new();
        while let Some(&Reverse((t, s))) = self.heap.peek() {
            if t > now_ms {
                break;
            }
            self.heap.pop();
            out.push(s);
        }
        out
    }
}
// ANCHOR_END: timers

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> Vec<(u32, i32)> {
        (0..100).map(|i| (i * 10, i as i32)).collect()
    }

    #[test]
    fn btree_and_sorted_vec_agree() {
        let v = log();
        let b: BTreeMap<u32, i32> = v.iter().copied().collect();
        for (from, to) in [(0, 1000), (15, 95), (990, 2000), (50, 50), (60, 10)] {
            if from <= to {
                assert_eq!(between(&b, from, to), between_sorted(&v, from, to), "{from}..{to}");
            } else {
                assert!(between_sorted(&v, from, to).is_empty());
            }
        }
    }

    #[test]
    fn window_wraps_without_growing() {
        let mut w = Window::new(4);
        for v in [10, 20, 30, 40, 50, 60] {
            w.push(v);
        }
        assert_eq!(w.mean(), Some(45));
        let (a, b) = w.as_slices();
        assert_eq!([a, b].concat(), [30, 40, 50, 60]);
        assert_eq!(w.capacity(), 4);
    }

    #[test]
    fn timers_fire_in_order() {
        let mut t = Timers::default();
        for (d, s) in [(300, 3), (100, 1), (200, 2), (100, 4)] {
            t.at(d, s);
        }
        assert_eq!(t.due(150), [1, 4]);
        assert_eq!(t.due(1000), [2, 3]);
        assert!(t.due(5000).is_empty());
    }
}
