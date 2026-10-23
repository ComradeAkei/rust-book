//! Раздел 20.6: цена многопоточности — ложное разделение. Кэш когерентен по
//! строкам (обычно 64 байта), а не по переменным. Если два потока пишут в
//! разные счётчики, попавшие в одну строку, каждая запись выбивает строку из
//! кэша соседа — переменные независимы, а торможение как от общей. Лекарство
//! — развести «горячие» данные по разным строкам выравниванием.

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

// ANCHOR: padded
/// Обёртка, занимающая целую кэш-линию: `#[repr(align(64))]` гарантирует, что
/// каждый экземпляр начинается с новой строки, а значит, соседи по массиву не
/// делят строку и не мешают друг другу.
#[repr(align(64))]
pub struct CachePadded<T>(pub T);

/// Каждый поток инкрементит СВОЙ счётчик (своя кэш-линия — нет ложного
/// разделения), в конце суммируем. Результат тот же, что у одного общего
/// счётчика под замком, но без борьбы за строку кэша.
pub fn per_thread_counters(n: usize, iters: u64) -> u64 {
    let counters: Vec<CachePadded<AtomicU64>> =
        (0..n).map(|_| CachePadded(AtomicU64::new(0))).collect();
    thread::scope(|s| {
        for c in &counters {
            s.spawn(move || {
                for _ in 0..iters {
                    // Relaxed достаточно: ни с чем не синхронизируемся, просто
                    // считаем (подробнее про Ordering — глава 21).
                    c.0.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });
    counters.iter().map(|c| c.0.load(Ordering::Relaxed)).sum()
}
// ANCHOR_END: padded

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_puts_each_counter_on_its_own_line() {
        // Выравнивание — целая кэш-линия, размер кратен ей.
        assert_eq!(align_of::<CachePadded<AtomicU64>>(), 64);
        assert_eq!(size_of::<CachePadded<AtomicU64>>(), 64);
    }

    #[test]
    fn counters_sum_correctly() {
        assert_eq!(per_thread_counters(8, 10_000), 80_000);
    }
}
