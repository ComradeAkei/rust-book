// Общие тесты главы 20 для задач A–D: подключаются через include! в крейты
// заготовок и решений после строки `use <крейт> as sut;`.
//
// Глава хостовая (потоки ОС). Задача C (прямой futex) — только Linux/x86-64
// и помечена ignore под Miri (Miri не исполняет syscall через asm!).

use std::sync::Arc;
use std::thread;

// --- Задача A: SendBox передаёт владение в поток ---

#[test]
fn a_sendbox_moves_ownership_across_threads() {
    let boxed = sut::a_send::SendBox::new(vec![1i32, 2, 3, 4]);
    // SendBox: Send — его можно переместить в другой поток.
    let data = thread::spawn(move || boxed.into_inner()).join().unwrap();
    assert_eq!(data, vec![1, 2, 3, 4]);
}

#[test]
fn a_sendbox_drops_without_leak() {
    // Создание и уничтожение без into_inner не должно течь (проверяет Miri).
    let _ = sut::a_send::SendBox::new(String::from("на выброс"));
}

// --- Задача B: параллельный подсчёт слов ---

#[test]
fn b_wordcount_matches_sequential() {
    let text = "a b c\nb c d\nc d a\na a a\n".repeat(50);
    // Эталон — последовательный подсчёт.
    let mut expected = std::collections::HashMap::new();
    for w in text.split_whitespace() {
        *expected.entry(w.to_owned()).or_insert(0usize) += 1;
    }
    for workers in [1, 2, 3, 8] {
        assert_eq!(sut::b_wordcount::word_frequencies(&text, workers), expected, "workers={workers}");
    }
}

// --- Задача C: мьютекс на futex (Linux/x86-64) ---

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
#[cfg_attr(miri, ignore = "прямой futex через syscall")]
fn c_futex_mutex_provides_mutual_exclusion() {
    use std::sync::atomic::{AtomicU64, Ordering};
    // Счётчик инкрементим НЕатомарно (load+store) под замком: верный итог
    // возможен только при настоящем взаимном исключении.
    let shared = Arc::new((sut::c_futex::FutexMutex::new(), AtomicU64::new(0)));
    let threads = 8;
    let iters = 20_000u64;
    thread::scope(|s| {
        for _ in 0..threads {
            let shared = Arc::clone(&shared);
            s.spawn(move || {
                let (m, c) = &*shared;
                for _ in 0..iters {
                    m.lock();
                    let v = c.load(Ordering::Relaxed);
                    c.store(v + 1, Ordering::Relaxed);
                    m.unlock();
                }
            });
        }
    });
    assert_eq!(shared.1.load(Ordering::Relaxed), threads * iters);
}

// --- Задача D: перевод без взаимной блокировки ---

#[test]
fn d_ordered_transfers_conserve_total_without_deadlock() {
    let bank = Arc::new(sut::d_deadlock::Bank::new(&[1000, 1000, 1000, 1000]));
    let total_before = bank.total();
    thread::scope(|s| {
        // Встречные переводы между одними и теми же счетами — классический
        // рецепт взаимной блокировки при неупорядоченном захвате.
        for _ in 0..8 {
            let bank = Arc::clone(&bank);
            s.spawn(move || {
                for _ in 0..5_000 {
                    bank.transfer(0, 3, 1);
                    bank.transfer(3, 0, 1);
                    bank.transfer(1, 2, 2);
                    bank.transfer(2, 1, 2);
                }
            });
        }
    });
    // Ни зависания (тест завершился), ни потери денег.
    assert_eq!(bank.total(), total_before);
}
