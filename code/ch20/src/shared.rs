//! Раздел 20.3: общее изменяемое состояние. `Send`/`Sync` пускают данные
//! через границу потока, но менять их из нескольких потоков всё равно нужно
//! под защитой. `Arc<T>` даёт общее владение (атомарный счётчик ссылок),
//! `Mutex<T>` — взаимное исключение, `RwLock<T>` — много читателей или один
//! писатель. Типичная связка общего изменяемого состояния — `Arc<Mutex<T>>`.

use std::sync::{Arc, Mutex, RwLock};
use std::thread;

// ANCHOR: counter
/// `n` потоков увеличивают общий счётчик по `iters` раз. `Arc` даёт каждому
/// потоку свою ручку к одному `Mutex`; `lock()` отдаёт охранника
/// (`MutexGuard`), пока он жив — доступ только у этого потока, по выходе из
/// области блокировка снимается.
pub fn concurrent_counter(n: usize, iters: usize) -> usize {
    let counter = Arc::new(Mutex::new(0usize));
    let handles: Vec<_> = (0..n)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..iters {
                    // Блокировка держится ровно на одну операцию: lock()
                    // возвращает охранника, он снимает замок в конце строки.
                    *counter.lock().unwrap() += 1;
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    // Все потоки завершились — ручка одна, блокировка свободна.
    Arc::into_inner(counter).unwrap().into_inner().unwrap()
}
// ANCHOR_END: counter

// ANCHOR: rwlock
/// `RwLock` разрешает либо много читателей одновременно, либо одного
/// писателя. Здесь — одна запись и параллельные чтения.
pub fn rwlock_demo() -> i32 {
    let data = Arc::new(RwLock::new(0i32));
    {
        // Единственный писатель: эксклюзивный доступ.
        *data.write().unwrap() = 100;
    }
    let readers: Vec<_> = (0..4)
        .map(|_| {
            let data = Arc::clone(&data);
            // Читатели не мешают друг другу: read() можно держать параллельно.
            thread::spawn(move || *data.read().unwrap())
        })
        .collect();
    readers.into_iter().map(|h| h.join().unwrap()).sum::<i32>()
}
// ANCHOR_END: rwlock

// ANCHOR: poison
/// Отравление: если поток паникует, держа блокировку, `Mutex`
/// «отравляется» — его состояние, возможно, оставлено в середине изменения.
/// Дальнейший `lock()` вернёт `Err(PoisonError)`, из которого всё же можно
/// достать данные (`into_inner`), приняв решение осознанно.
pub fn read_through_poison() -> i32 {
    let m = Arc::new(Mutex::new(0i32));
    let m2 = Arc::clone(&m);
    // Поток паникует, держа замок: мьютекс отравлен.
    let _ = thread::spawn(move || {
        let mut g = m2.lock().unwrap();
        *g = 1;
        panic!("паника под замком");
    })
    .join();
    // lock() теперь даёт Err; забираем данные из отравленного охранника.
    match m.lock() {
        Ok(g) => *g,
        Err(poisoned) => *poisoned.into_inner(),
    }
}
// ANCHOR_END: poison

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_sums_all_increments() {
        assert_eq!(concurrent_counter(8, 1000), 8000);
    }

    #[test]
    fn rwlock_readers_see_the_write() {
        assert_eq!(rwlock_demo(), 400); // 4 читателя по 100
    }

    #[test]
    fn poison_still_yields_data() {
        assert_eq!(read_through_poison(), 1);
    }
}
