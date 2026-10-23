//! Раздел 21.1: атомарные типы и операции. `AtomicUsize`, `AtomicU64`,
//! `AtomicBool`, `AtomicPtr` ведут себя как обычные ячейки, но их операции
//! неделимы: `load`, `store`, `swap`, `fetch_add` и семейство, а также
//! `compare_exchange` — сравнить-и-обменять, основа всего lock-free.
//! Атомарность и порядок — разные вещи: порядок задаёт `Ordering`
//! ([21.2](super::ordering)); здесь берём то, что для счётчика хватает.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread;

// ANCHOR: counter
/// Lock-free счётчик: `fetch_add` неделимо прибавляет и возвращает прежнее
/// значение. Восемь потоков без всякого замка — ни гонки, ни потерянных
/// инкрементов, потому что сложение атомарно.
pub fn lockfree_counter(threads: usize, iters: u64) -> u64 {
    let counter = AtomicU64::new(0);
    thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                for _ in 0..iters {
                    // Relaxed: нам важна только атомарность суммы, порядок
                    // с другими переменными ни при чём ([21.3]).
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });
    counter.load(Ordering::Relaxed)
}
// ANCHOR_END: counter

// ANCHOR: cas
/// Взять «один раз»: первый поток, сменивший флаг `false → true`, выигрывает.
/// `compare_exchange` меняет значение, только если оно равно ожидаемому, и
/// сообщает, удалось ли, — ровно один поток получит `true`.
pub fn take_once(flag: &std::sync::atomic::AtomicBool) -> bool {
    flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_ok()
}

/// Атомарный максимум через цикл `compare_exchange_weak`. Классический
/// рисунок lock-free: прочитать, посчитать новое, попытаться записать, если
/// значение не изменилось; иначе повторить. `_weak` вправе ложно не
/// сработать (на ARM — из-за `strex`), поэтому его всегда зовут в цикле.
pub fn fetch_max(cell: &AtomicUsize, value: usize) -> usize {
    let mut current = cell.load(Ordering::Relaxed);
    loop {
        if value <= current {
            return current; // уже не меньше — менять нечего
        }
        match cell.compare_exchange_weak(current, value, Ordering::AcqRel, Ordering::Relaxed) {
            Ok(prev) => return prev,
            Err(actual) => current = actual, // кто-то успел раньше — повторяем
        }
    }
}
// ANCHOR_END: cas

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn counter_loses_nothing() {
        assert_eq!(lockfree_counter(8, 10_000), 80_000);
    }

    #[test]
    fn take_once_has_single_winner() {
        let flag = AtomicBool::new(false);
        let winners: usize = thread::scope(|s| {
            let hs: Vec<_> = (0..8).map(|_| s.spawn(|| take_once(&flag) as usize)).collect();
            hs.into_iter().map(|h| h.join().unwrap()).sum()
        });
        assert_eq!(winners, 1); // ровно один поток выиграл
    }

    #[test]
    fn fetch_max_converges_to_maximum() {
        let cell = AtomicUsize::new(0);
        let cell = &cell; // ссылку (Copy) можно отдать каждому потоку
        thread::scope(|s| {
            for v in [3usize, 9, 1, 7, 42, 5] {
                s.spawn(move || {
                    fetch_max(cell, v);
                });
            }
        });
        assert_eq!(cell.load(Ordering::Relaxed), 42);
    }
}
