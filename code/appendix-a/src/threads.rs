//! Раздел A.9: потоки и синхронизация.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

// ANCHOR: mutex
/// В C мьютекс и данные живут отдельно, и связь между ними держится на
/// дисциплине. Здесь данные лежат внутри `Mutex`, а доступ к ним есть только
/// через guard, который отпускает мьютекс в деструкторе.
pub fn parallel_sum(data: &[u64], threads: usize) -> u64 {
    let total = Mutex::new(0u64);
    let chunk = data.len().div_ceil(threads.max(1)).max(1);
    thread::scope(|s| {
        for part in data.chunks(chunk) {
            let total = &total;
            s.spawn(move || {
                let sum: u64 = part.iter().sum();
                *total.lock().unwrap() += sum;
            });
        }
    }); // scope дожидается всех потоков, как pthread_join для каждого
    total.into_inner().unwrap()
}
// ANCHOR_END: mutex

// ANCHOR: atomic
/// `static _Atomic uint64_t packets;` и `atomic_fetch_add_explicit(..., memory_order_relaxed)`.
pub static PACKETS: AtomicU64 = AtomicU64::new(0);

pub fn count_packet() -> u64 {
    PACKETS.fetch_add(1, Ordering::Relaxed) + 1
}
// ANCHOR_END: atomic

/// Гонка данных не компилируется: `Rc` нельзя передать в другой поток.
///
/// ```compile_fail,E0277
/// use std::rc::Rc;
/// let shared = Rc::new(5);
/// let copy = Rc::clone(&shared);
/// std::thread::spawn(move || println!("{copy}"));
/// ```
pub const E0277_IS_CHECKED_BY_DOCTEST: () = ();

/* ANCHOR: rc_race
let shared = Rc::new(5);
let copy = Rc::clone(&shared);
std::thread::spawn(move || println!("{copy}"));
// error[E0277]: `Rc<i32>` cannot be sent between threads safely
ANCHOR_END: rc_race */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_in_threads() {
        let data: Vec<u64> = (1..=1000).collect();
        assert_eq!(parallel_sum(&data, 4), 500_500);
        assert_eq!(parallel_sum(&[], 4), 0);
    }

    #[test]
    fn atomic_counter() {
        let before = PACKETS.load(Ordering::Relaxed);
        thread::scope(|s| {
            for _ in 0..4 {
                s.spawn(|| {
                    for _ in 0..100 {
                        count_packet();
                    }
                });
            }
        });
        assert_eq!(PACKETS.load(Ordering::Relaxed) - before, 400);
    }
}
