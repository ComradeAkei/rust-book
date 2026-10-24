//! Задача A: пять замеров, в каждом — одна ошибка методики. Функции
//! `bench_N` — как их написал автор, `fixed_N` — ваши исправленные. Все
//! возвращают наносекунды на одну операцию; `iters` — сколько раз
//! повторить работу. Замеры запускает программа `ch30-task`:
//! `cargo run --release -p ch30-exercises --bin ch30-task -- a all 100`.

use std::hint::black_box;
use std::time::Instant;

// ANCHOR: mistake
/// Ошибки методики из раздела 30.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mistake {
    /// Результат работы никуда не идёт, и оптимизатор её выбросил.
    DiscardedResult,
    /// Вход известен при компиляции, и ответ посчитан заранее.
    ConstantFolding,
    /// Одна выборка на холодных данных: страничные сбои и промахи
    /// первого прохода вместо работы.
    ColdStart,
    /// Замер собран без оптимизаций.
    DebugBuild,
    /// В замер попала подготовка входа: выделение и заполнение.
    SetupInTimedRegion,
}
// ANCHOR_END: mistake

// ANCHOR: work
/// FNV-1a: хеш, который считает каждый байт по цепочке умножений.
pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Шестьдесят четыре байта, известные при компиляции.
pub static KEY: [u8; 64] = {
    let mut a = [0u8; 64];
    let mut i = 0;
    while i < 64 {
        a[i] = (i * 37 + 11) as u8;
        i += 1;
    }
    a
};

/// Данные для замеров: `n` псевдослучайных байт.
pub fn input(n: usize) -> Vec<u8> {
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 56) as u8
        })
        .collect()
}

/// Умножить каждый байт на три — на месте.
pub fn triple(data: &mut [u8]) {
    for b in data {
        *b = b.wrapping_mul(3);
    }
}

fn per_op(start: Instant, ops: u64) -> f64 {
    start.elapsed().as_nanos() as f64 / ops as f64
}
// ANCHOR_END: work

// ANCHOR: benches
/// Хеш 4 КиБ, наносекунды на байт.
pub fn bench_1(iters: u32) -> f64 {
    let start = Instant::now();
    for _ in 0..iters {
        let data = input(4096);
        black_box(fnv1a(&data));
    }
    per_op(start, u64::from(iters) * 4096)
}

/// Хеш 4 КиБ, наносекунды на байт.
pub fn bench_2(iters: u32) -> f64 {
    let data = input(4096);
    let start = Instant::now();
    for _ in 0..iters {
        fnv1a(&data);
    }
    per_op(start, u64::from(iters) * 4096)
}

/// Во сколько раз сумма по индексам медленнее суммы итератором.
// Цикл по индексу и есть предмет замера, итератор clippy здесь не нужен.
#[allow(clippy::needless_range_loop)]
pub fn bench_3(iters: u32) -> f64 {
    let v: Vec<u32> = (0..4096).collect();
    let start = Instant::now();
    for _ in 0..iters {
        let mut s = 0u32;
        for i in 0..v.len() {
            s = s.wrapping_add(v[i]);
        }
        black_box(s);
    }
    let index = start.elapsed().as_nanos() as f64;
    let start = Instant::now();
    for _ in 0..iters {
        black_box(v.iter().fold(0u32, |s, &x| s.wrapping_add(x)));
    }
    index / start.elapsed().as_nanos() as f64
}

/// Хеш ключа из 64 байт, наносекунды на вызов.
pub fn bench_4(iters: u32) -> f64 {
    let start = Instant::now();
    for _ in 0..iters {
        black_box(fnv1a(&KEY));
    }
    per_op(start, u64::from(iters))
}

/// `triple` по 64 МиБ, наносекунды на байт.
pub fn bench_5(_iters: u32) -> f64 {
    let mut data = vec![0u8; 64 << 20];
    let start = Instant::now();
    triple(black_box(&mut data));
    per_op(start, data.len() as u64)
}
// ANCHOR_END: benches

// ANCHOR: answers
/// Ошибка методики в замерах `bench_1` … `bench_5`, по порядку.
pub fn mistakes() -> [Mistake; 5] {
    todo!("30.A: какая ошибка в каждом замере")
}
// ANCHOR_END: answers

// ANCHOR: fixed
/// `bench_1` без ошибки.
pub fn fixed_1(iters: u32) -> f64 {
    let _ = iters;
    todo!("30.A: fixed_1")
}

/// `bench_2` без ошибки.
pub fn fixed_2(iters: u32) -> f64 {
    let _ = iters;
    todo!("30.A: fixed_2")
}

/// `bench_3` без ошибки. Если в этой сборке сравнение не имеет смысла,
/// замер отказывается мерить и возвращает `None`.
pub fn fixed_3(iters: u32) -> Option<f64> {
    let _ = iters;
    todo!("30.A: fixed_3")
}

/// `bench_4` без ошибки.
pub fn fixed_4(iters: u32) -> f64 {
    let _ = iters;
    todo!("30.A: fixed_4")
}

/// `bench_5` без ошибки: `iters` — число выборок.
pub fn fixed_5(iters: u32) -> f64 {
    let _ = iters;
    todo!("30.A: fixed_5")
}
// ANCHOR_END: fixed
