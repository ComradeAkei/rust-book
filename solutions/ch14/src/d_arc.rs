//! Задача D: цена `Arc::clone`, когда клонируют из многих потоков.
//!
//! Каждая функция запускает `threads` потоков, каждый делает `iters` пар
//! `clone` + `drop` (или столько же чтений значения) и возвращает время
//! одной пары в наносекундах у самого медленного потока. Каждый поток
//! засекает время сам: главный поток после барьера может не получить
//! процессор, пока работа не кончится. Свой клон каждый поток берёт до
//! замера и держит на своём стеке: иначе все потоки читали бы указатели из
//! одного вектора, и линия вектора, а не счётчика, стала бы общей.

use alloc::sync::Arc;
use alloc::vec::Vec;
use std::hint::black_box;
use std::sync::Barrier;
use std::time::Instant;

/// Запускает `threads` потоков с работой `work(номер)` одновременно и
/// возвращает наносекунд на одну из `iters` итераций у самого медленного.
fn timed(threads: usize, iters: usize, work: impl Fn(usize) + Sync) -> f64 {
    let start = Barrier::new(threads);
    std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|i| {
                let (start, work) = (&start, &work);
                s.spawn(move || {
                    start.wait();
                    let t = Instant::now();
                    work(i);
                    t.elapsed()
                })
            })
            .collect();
        let slowest = hs.into_iter().map(|h| h.join().unwrap()).max().unwrap();
        slowest.as_secs_f64() * 1e9 / iters as f64
    })
}

// ANCHOR: shared
/// Один `Arc` на всех: все потоки меняют один и тот же счётчик.
pub fn shared(threads: usize, iters: usize) -> f64 {
    let arc = Arc::new(0u64);
    timed(threads, iters, |_| {
        let mine = Arc::clone(&arc);
        for _ in 0..iters {
            drop(black_box(Arc::clone(black_box(&mine))));
        }
    })
}
// ANCHOR_END: shared

// ANCHOR: private
/// У каждого потока свой `Arc`: счётчики разные, но блоки по 24 байта
/// распределитель кладёт где придётся, и два из них могут оказаться в
/// одной кэш-линии.
pub fn private(threads: usize, iters: usize) -> f64 {
    private_lines(threads, iters).0
}

/// То же и номера кэш-линий (адрес / 64), в которых лежат счётчики.
pub fn private_lines(threads: usize, iters: usize) -> (f64, Vec<usize>) {
    let arcs: Vec<_> = (0..threads).map(|_| Arc::new(0u64)).collect();
    let lines = arcs.iter().map(|a| (Arc::as_ptr(a) as usize - 16) / 64).collect();
    let ns = timed(threads, iters, |i| {
        let mine = Arc::clone(&arcs[i]);
        for _ in 0..iters {
            drop(black_box(Arc::clone(black_box(&mine))));
        }
    });
    (ns, lines)
}

/// То же, но значение выровнено на 64 байта: каждый блок `Arc`
/// начинается со своей кэш-линии, и счётчики соседей не делят линию.
#[repr(align(64))]
pub struct Padded(pub u64);

pub fn private_padded(threads: usize, iters: usize) -> f64 {
    let arcs: Vec<_> = (0..threads).map(|_| Arc::new(Padded(0))).collect();
    timed(threads, iters, |i| {
        let mine = Arc::clone(&arcs[i]);
        for _ in 0..iters {
            drop(black_box(Arc::clone(black_box(&mine))));
        }
    })
}
// ANCHOR_END: private

// ANCHOR: borrowed
/// Без клонов: потоки `scope` читают значение по обычной ссылке, и
/// счётчик никто не трогает.
pub fn borrowed(threads: usize, iters: usize) -> f64 {
    let arc = Arc::new(0u64);
    let value: &u64 = &arc;
    timed(threads, iters, |_| {
        for _ in 0..iters {
            black_box(*black_box(value));
        }
    })
}
// ANCHOR_END: borrowed

/// Номера кэш-линий (адрес / 64) счётчиков `n` блоков `Arc`, выделенных
/// так же, как в `private` и `private_padded`.
pub fn lines(n: usize, padded: bool) -> Vec<usize> {
    if padded {
        let a: Vec<_> = (0..n).map(|_| Arc::new(Padded(0))).collect();
        // Значение выровнено на 64, и счётчики — в линии перед ним.
        a.iter().map(|a| (Arc::as_ptr(a) as usize - 64) / 64).collect()
    } else {
        // Счётчики лежат за 16 байт до значения.
        let a: Vec<_> = (0..n).map(|_| Arc::new(0u64)).collect();
        a.iter().map(|a| (Arc::as_ptr(a) as usize - 16) / 64).collect()
    }
}
