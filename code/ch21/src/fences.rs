//! Раздел 21.3: Relaxed, SeqCst и барьеры. `Relaxed` даёт атомарность без
//! всякого порядка. Release/Acquire ([21.2](super::ordering)) упорядочивают
//! пару «писатель → читатель». Но есть сценарии, где нужен единый
//! глобальный порядок для ВСЕХ потоков сразу, — его даёт `SeqCst`. Классика
//! — буфер записи (store buffering): две переменные, два потока, и исход, где
//! каждый «не увидел» запись другого.

use std::sync::atomic::{AtomicUsize, Ordering, fence};
use std::thread;

// ANCHOR: seqcst
/// Проба на буфер записи. Поток 1: `x = 1; r1 = y`. Поток 2: `y = 1; r2 = x`.
/// Со слабым порядком возможен исход `r1 == 0 && r2 == 0` (каждый прочитал
/// переменную до чужой записи — записи «застряли» в буфере процессора). С
/// `SeqCst` существует единый порядок всех операций, и такой исход
/// невозможен: хотя бы один поток увидит чужую единицу. Возвращает число
/// раундов с исходом `(0, 0)` — для `SeqCst` это ноль.
pub fn store_buffering_both_zero(rounds: usize) -> usize {
    let mut both_zero = 0;
    for _ in 0..rounds {
        let x = AtomicUsize::new(0);
        let y = AtomicUsize::new(0);
        let (r1, r2) = thread::scope(|s| {
            let t1 = s.spawn(|| {
                x.store(1, Ordering::SeqCst);
                y.load(Ordering::SeqCst)
            });
            let r2 = {
                y.store(1, Ordering::SeqCst);
                x.load(Ordering::SeqCst)
            };
            (t1.join().unwrap(), r2)
        });
        if r1 == 0 && r2 == 0 {
            both_zero += 1;
        }
    }
    both_zero
}
// ANCHOR_END: seqcst

// ANCHOR: fence
/// Отдельный барьер вместо порядка на самой операции. `fence(Release)` после
/// обычных Relaxed-записей и `fence(Acquire)` перед Relaxed-чтениями дают ту
/// же happens-before, что Release-store и Acquire-load, но барьер отделён от
/// конкретной переменной — удобно, когда публикуемых данных несколько.
pub fn publish_with_fence(data: &AtomicUsize, flag: &std::sync::atomic::AtomicBool) {
    data.store(99, Ordering::Relaxed);
    fence(Ordering::Release); // всё выше — до флага
    flag.store(true, Ordering::Relaxed);
}
// ANCHOR_END: fence

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn seqcst_forbids_both_zero() {
        // При SeqCst исход (0, 0) не встречается ни разу.
        assert_eq!(store_buffering_both_zero(2000), 0);
    }

    #[test]
    fn fence_publishes_data() {
        let data = AtomicUsize::new(0);
        let flag = AtomicBool::new(false);
        thread::scope(|s| {
            s.spawn(|| publish_with_fence(&data, &flag));
            while !flag.load(Ordering::Acquire) {
                std::hint::spin_loop();
            }
            assert_eq!(data.load(Ordering::Relaxed), 99);
        });
    }
}
