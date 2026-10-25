//! Раздел 31.5: опыты для санитайзеров. Каждый — ошибка, которую
//! компилятор пропустил, потому что она в `unsafe` или по ту сторону FFI.
//! Без санитайзера программа чаще всего отрабатывает «нормально».
//!
//! Запуск: `san <опыт>`, опыты — uaf, overflow, ffi, leak, race, spsc,
//! relaxed, once. Последний ошибки не содержит: на нём TSan ошибается
//! сам, если std собрана без него.

use ch31::spsc::{AcqRel, AllRelaxed, Orderings, Spsc};
use std::cell::UnsafeCell;
use std::sync::{Arc, OnceLock};
use std::thread;

// ANCHOR: uaf
/// Указатель на элемент вектора пережил перераспределение буфера.
fn uaf() {
    let mut v = vec![1u32, 2, 3];
    let first: *const u32 = &v[0];
    v.extend(4..100);
    // SAFETY: нарушено — extend мог перенести буфер, first недействителен.
    println!("первый элемент: {}", unsafe { *first });
}
// ANCHOR_END: uaf

// ANCHOR: overflow
/// Чтение за концом без проверки границ. Индекс приходит «снаружи»:
/// будь он константой, LLVM вывел бы из `get_unchecked`, что этот код
/// недостижим, и вместо чтения поставил бы ud2.
fn overflow() {
    let v = vec![7u8; 16];
    let i = std::hint::black_box(16);
    // SAFETY: нарушено — индекс 16 вне вектора длины 16.
    println!("v[{i}] = {}", unsafe { *v.get_unchecked(i) });
}
// ANCHOR_END: overflow

// ANCHOR: ffi
unsafe extern "C" {
    /// Пишет в `buf` числа 0, 1, 2… — по контракту `len` штук.
    fn fill_counter(buf: *mut u8, len: usize);
}

fn ffi() {
    let mut v = vec![0u8; 16];
    // SAFETY: по контракту — buf на len байт; C-сторона его нарушает.
    unsafe { fill_counter(v.as_mut_ptr(), v.len()) };
    println!("последний: {}", v[15]);
}
// ANCHOR_END: ffi

// ANCHOR: leak
/// Утечка безопасна в смысле Rust, но LeakSanitizer её видит.
fn leak() {
    // black_box не даёт LLVM выбросить ненужное выделение целиком.
    std::mem::forget(std::hint::black_box(vec![0u8; 1000]));
    println!("вектор забыт");
}
// ANCHOR_END: leak

// ANCHOR: race
/// Обёртка, которой вручную разрешили делиться между потоками.
struct Racy(UnsafeCell<u64>);
// SAFETY: нарушено — доступ ничем не синхронизирован.
unsafe impl Sync for Racy {}

fn race() {
    let counter = Arc::new(Racy(UnsafeCell::new(0)));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let c = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..100_000 {
                    // SAFETY: нарушено — второй поток пишет туда же.
                    unsafe { *c.0.get() += 1 };
                }
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
    // SAFETY: потоки завершены и присоединены.
    println!("счётчик: {} из 200000", unsafe { *counter.0.get() });
}
// ANCHOR_END: race

// ANCHOR: spsc
/// Очередь главы 21 из двух потоков: с порядками AcqRel гонок нет, со
/// всеми Relaxed чтение слота не упорядочено с записью в него. Ждут через
/// `yield_now`: с пустым циклом ожидания TSan на загруженной машине теряет
/// отчёт (раздел 31.5, ловушка в конце).
fn spsc<O: Orderings + Send + Sync + 'static>() {
    let q = Arc::new(Spsc::<u32, 8, O>::new());
    let producer = {
        let q = Arc::clone(&q);
        thread::spawn(move || {
            for i in 0..10_000 {
                // SAFETY: писатель — только этот поток.
                while unsafe { q.push(i) }.is_err() {
                    thread::yield_now();
                }
            }
        })
    };
    let mut sum = 0u64;
    let mut got = 0;
    while got < 10_000 {
        // SAFETY: читатель — только главный поток.
        match unsafe { q.pop() } {
            Some(v) => {
                sum += u64::from(v);
                got += 1;
            }
            None => thread::yield_now(),
        }
    }
    producer.join().unwrap();
    println!("сумма: {sum}");
}
// ANCHOR_END: spsc

// ANCHOR: once
/// Таблицу строит тот, кому она понадобилась первым. Ошибки нет:
/// `OnceLock` публикует значение Release-записью внутри std, а `get`
/// читает его состояние Acquire-загрузкой.
static TABLE: OnceLock<Vec<u32>> = OnceLock::new();

fn once() {
    let builder = thread::spawn(|| TABLE.get_or_init(|| (0..1000).collect()).len());
    let table = loop {
        if let Some(t) = TABLE.get() {
            break t;
        }
        std::hint::spin_loop();
    };
    println!("{}", table[999]);
    assert_eq!(builder.join().unwrap(), 1000);
}
// ANCHOR_END: once

fn main() {
    let what = std::env::args().nth(1).unwrap_or_default();
    match what.as_str() {
        "uaf" => uaf(),
        "overflow" => overflow(),
        "ffi" => ffi(),
        "leak" => leak(),
        "race" => race(),
        "spsc" => spsc::<AcqRel>(),
        "relaxed" => spsc::<AllRelaxed>(),
        "once" => once(),
        _ => {
            eprintln!("опыты: uaf overflow ffi leak race spsc relaxed once");
            std::process::exit(2);
        }
    }
}
