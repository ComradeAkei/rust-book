//! Раздел 31.6: loom перебирает чередования двух потоков над очередью
//! главы 21. Запуск: RUSTFLAGS="--cfg loom" cargo test -p ch31 --release
//! --test loom --target-dir target/loom (свой каталог сборки: флаг
//! меняет все крейты, и обычная сборка не пересобирается из-за него).
#![cfg(loom)]

use ch31::spsc::{AcqRel, AllRelaxed, Orderings, Spsc};
use loom::sync::Arc;
use loom::thread;
use std::sync::atomic::{AtomicUsize, Ordering};

// ANCHOR: model
/// Писатель кладёт два элемента в очередь на три слота, читатель ждёт
/// оба, уступая процессор. Сколько исполнений перебрал loom — в `runs`.
fn two_items<O: Orderings + 'static>(runs: &'static AtomicUsize) {
    loom::model(move || {
        runs.fetch_add(1, Ordering::Relaxed);
        let q = Arc::new(Spsc::<u32, 3, O>::new());
        let producer = {
            let q = Arc::clone(&q);
            thread::spawn(move || {
                q.push(1).unwrap();
                q.push(2).unwrap();
            })
        };
        let mut got = Vec::new();
        while got.len() < 2 {
            match q.pop() {
                Some(v) => got.push(v),
                None => thread::yield_now(),
            }
        }
        producer.join().unwrap();
        assert_eq!(got, [1, 2]);
    });
}

#[test]
fn acq_rel_is_sound() {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    two_items::<AcqRel>(&RUNS);
    println!("AcqRel: {} исполнений", RUNS.load(Ordering::Relaxed));
    assert!(RUNS.load(Ordering::Relaxed) > 50);
}

#[test]
#[should_panic(expected = "Causality violation: Concurrent read and write accesses.")]
fn all_relaxed_is_caught() {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    two_items::<AllRelaxed>(&RUNS);
}
// ANCHOR_END: model

// ANCHOR: once
/// Модель без ожидания: читатель забирает один раз и сразу ждёт писателя.
/// loom 0.7.2 проходит её за одно исполнение — и ошибку не находит.
#[test]
fn single_pop_model_explores_once() {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    loom::model(|| {
        RUNS.fetch_add(1, Ordering::Relaxed);
        let q = Arc::new(Spsc::<u32, 3, AllRelaxed>::new());
        let producer = {
            let q = Arc::clone(&q);
            thread::spawn(move || q.push(1).unwrap())
        };
        let first = q.pop();
        producer.join().unwrap();
        assert_eq!(first.or_else(|| q.pop()), Some(1));
    });
    assert_eq!(RUNS.load(Ordering::Relaxed), 1);
}
// ANCHOR_END: once
