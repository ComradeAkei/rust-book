// Общие тесты задачи 15.B: подключаются через include! в крейты заготовок
// и решений, после строки `use <крейт> as sut;`. Распределитель-счётчик
// студента стоит на весь тестовый процесс, а тесты идут параллельно, в
// разных потоках: `measure` обязан видеть только свой поток.

use std::hint::black_box;
use sut::b_count::{Counting, Stats, measure};

#[global_allocator]
static COUNTING: Counting = Counting;

/// Большой блок: под Miri поменьше, чтобы заполнение не шло минутами.
const BIG: usize = if cfg!(miri) { 1 << 17 } else { 1 << 20 };

#[test]
fn task_b_vec_growth() {
    let (v, s) = measure(|| {
        let mut v = Vec::new();
        for i in 0..17u32 {
            v.push(i);
        }
        v
    });
    // 4, 8, 16, 32 элемента по 4 байта: блок растёт realloc-ом.
    assert_eq!(s, Stats { allocs: 1, reallocs: 3, deallocs: 0, requested: 128, peak: 128, retained: 128 });
    let ((), s) = measure(|| drop(v));
    assert_eq!(s, Stats { allocs: 0, reallocs: 0, deallocs: 1, requested: 0, peak: 0, retained: -128 });
}

#[test]
fn task_b_peak_is_not_retained() {
    let (sum, s) = measure(|| {
        let big = vec![1u8; BIG];
        let small = black_box(vec![0u64; 16]);
        big.iter().map(|&b| b as u64).sum::<u64>() + small.len() as u64
    });
    assert_eq!(sum, BIG as u64 + 16);
    assert_eq!((s.allocs, s.deallocs, s.retained), (2, 2, 0));
    assert_eq!(s.peak, BIG as u64 + 128);
    assert_eq!(s.requested, BIG as u64 + 128);
}

#[test]
fn task_b_zeroed_and_zero_sized() {
    let (_, s) = measure(|| drop(black_box(vec![0u32; 1000])));
    assert_eq!((s.allocs, s.deallocs, s.peak), (1, 1, 4000));
    let (_, s) = measure(|| {
        let b = Box::new(());
        let v: Vec<()> = vec![(); 1000];
        black_box((b, v));
    });
    assert_eq!(s, Stats::default());
}

#[test]
fn task_b_shrink_lowers_live() {
    let mut v: Vec<u8> = Vec::with_capacity(1000);
    v.extend_from_slice(b"adc0");
    let ((), s) = measure(|| v.shrink_to_fit());
    assert_eq!((s.reallocs, s.requested, s.peak, s.retained), (1, 0, 0, -996));
}

#[test]
fn task_b_nested() {
    let (inner, outer) = measure(|| {
        let a = black_box(vec![0u8; 100]);
        let ((), inner) = measure(|| drop(black_box(vec![0u8; 10_000])));
        drop(a);
        inner
    });
    assert_eq!((inner.allocs, inner.peak, inner.retained), (1, 10_000, 0));
    // Внешний замер видит и внутренний пик поверх своих 100 байт.
    assert_eq!((outer.allocs, outer.deallocs, outer.retained), (2, 2, 0));
    assert_eq!(outer.peak, 10_100);
}

#[test]
fn task_b_other_threads_not_counted() {
    let (n, s) = measure(|| {
        std::thread::spawn(|| black_box(vec![1u8; BIG]).len()).join().unwrap()
    });
    assert_eq!(n, BIG);
    // Сам запуск потока выделяет немного памяти в этом потоке, но большой блок
    // выделил и освободил другой.
    assert!(s.peak < 1 << 16, "{s:?}");
}

#[test]
fn task_b_parallel_tests_do_not_mix() {
    let handles: Vec<_> = (1..=4u64)
        .map(|k| {
            std::thread::spawn(move || {
                let (_, s) = measure(|| {
                    for _ in 0..100 {
                        drop(black_box(vec![0u8; 1000 * k as usize]));
                    }
                });
                (k, s)
            })
        })
        .collect();
    for h in handles {
        let (k, s) = h.join().unwrap();
        assert_eq!((s.allocs, s.deallocs, s.peak, s.requested), (100, 100, 1000 * k, 100_000 * k));
    }
}
