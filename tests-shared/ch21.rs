// Общие тесты главы 21 для задач A–D: подключаются через include! в крейты
// заготовок и решений после строки `use <крейт> as sut;`.
//
// Глава хостовая (потоки std). Многопоточные тесты помечены ignore под Miri
// (тысячи операций по кооперативному планировщику Miri идут слишком долго).

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// --- Задача A: спинлок ---

#[test]
#[cfg_attr(miri, ignore = "многопоточный стресс")]
fn a_spinlock_provides_mutual_exclusion() {
    let lock = Arc::new(sut::a_spinlock::SpinLock::new(0u64));
    let threads = 8;
    let iters = 20_000u64;
    thread::scope(|s| {
        for _ in 0..threads {
            let lock = Arc::clone(&lock);
            s.spawn(move || {
                for _ in 0..iters {
                    *lock.lock() += 1;
                }
            });
        }
    });
    assert_eq!(*lock.lock(), threads * iters);
}

// --- Задача B: справедливый ticket lock ---

#[test]
#[cfg_attr(miri, ignore = "многопоточный стресс")]
fn b_ticket_lock_provides_mutual_exclusion() {
    let lock = Arc::new(sut::b_ticket::TicketLock::new(0u64));
    let threads = 8;
    let iters = 20_000u64;
    thread::scope(|s| {
        for _ in 0..threads {
            let lock = Arc::clone(&lock);
            s.spawn(move || {
                for _ in 0..iters {
                    *lock.lock() += 1;
                }
            });
        }
    });
    assert_eq!(*lock.lock(), threads * iters);
}

// --- Задача C: буфер записи ---

#[test]
#[cfg_attr(miri, ignore = "многопоточный стресс")]
fn c_seqcst_forbids_both_zero() {
    // При SeqCst исход (0, 0) невозможен.
    assert_eq!(sut::c_store_buffering::count_both_zero(Ordering::SeqCst, 2000), 0);
    // При Relaxed исход допустим: просто проверяем, что код работает (частота
    // зависит от железа, поэтому жёстко её не фиксируем).
    let _ = sut::c_store_buffering::count_both_zero(Ordering::Relaxed, 100);
}

// --- Задача D: стек Трайбера ---

#[test]
#[cfg_attr(miri, ignore = "многопоточный стресс")]
fn d_treiber_conserves_sum_under_concurrency() {
    const THREADS: u64 = 4;
    const PER: u64 = 25_000;
    let stack: Arc<sut::d_treiber::TreiberStack<256>> = Arc::new(sut::d_treiber::TreiberStack::new());
    let produced: u64 = (0..THREADS * PER).sum();
    let consumed = Arc::new(AtomicU64::new(0));
    let popped = Arc::new(AtomicUsize::new(0));

    thread::scope(|s| {
        // Производители кладут числа 0..THREADS*PER.
        for t in 0..THREADS {
            let stack = Arc::clone(&stack);
            s.spawn(move || {
                for v in t * PER..(t + 1) * PER {
                    while stack.push(v).is_err() {
                        std::hint::spin_loop();
                    }
                }
            });
        }
        // Потребители снимают, пока не разобрали всё.
        for _ in 0..THREADS {
            let stack = Arc::clone(&stack);
            let consumed = Arc::clone(&consumed);
            let popped = Arc::clone(&popped);
            s.spawn(move || {
                while popped.load(Ordering::Relaxed) < (THREADS * PER) as usize {
                    if let Some(v) = stack.pop() {
                        consumed.fetch_add(v, Ordering::Relaxed);
                        popped.fetch_add(1, Ordering::Relaxed);
                    } else {
                        std::hint::spin_loop();
                    }
                }
            });
        }
    });

    assert_eq!(popped.load(Ordering::Relaxed), (THREADS * PER) as usize);
    assert_eq!(consumed.load(Ordering::Relaxed), produced);
}
