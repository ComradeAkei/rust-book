// Программа замеров задач главы 30.
//
// Файл не является крейтом: его подключают через `include!` программы
// `exercises/ch30/src/bin/ch30-task.rs` и `solutions/ch30/src/bin/ch30-task.rs`,
// заранее объявив `use <крейт> as sut;`. Тесты задач (tests-shared/ch30.rs)
// собирают её с --release и запускают сами — напрямую и под cachegrind.
//
// cargo run --release -p ch30-solutions --bin ch30-task -- a all 100
// cargo run --release -p ch30-solutions --bin ch30-task -- b soa 1048576
// cargo run --release -p ch30-solutions --bin ch30-task -- c fast 1073741824 3
// cargo run --release -p ch30-solutions --bin ch30-task -- d branchless random 1024

use ch30::measure::bench;
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::Instant;

// ANCHOR: alloc
/// Распределитель, который считает выделения, пока `COUNTING` включён, —
/// только в задаче A. Счётчик крейта telemetry-store (раздел 30.7) ведёт
/// пять атомарных операций на пару выделение — освобождение, а наивный
/// подсчёт n-грамм задачи C выделяет вектор на каждый байт текста:
/// счётчик удвоил бы его время.
struct Counting;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicU64 = AtomicU64::new(0);

fn count() {
    if COUNTING.load(Relaxed) {
        ALLOCS.fetch_add(1, Relaxed);
    }
}

// SAFETY: каждый вызов передаётся System с теми же аргументами; счётчик —
// атомик, он не выделяет память и не трогает выданные блоки.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: контракт alloc тот же, что у вызывающего.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: контракт alloc_zeroed тот же, что у вызывающего.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: контракт realloc тот же, что у вызывающего.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTED: Counting = Counting;
// ANCHOR_END: alloc

/// Аргумент `i` командной строки или `default`.
fn arg<T: std::str::FromStr>(i: usize, default: T) -> T {
    std::env::args()
        .nth(i)
        .map_or(default, |s| s.parse().unwrap_or_else(|_| panic!("аргумент {i}: не число: {s}")))
}

fn main() {
    let task = std::env::args().nth(1).expect("задача: a, b, c или d");
    let what = std::env::args().nth(2).unwrap_or_default();
    match task.as_str() {
        "a" => task_a(&what),
        "b" => task_b(&what),
        "c" => task_c(&what),
        "d" => task_d(&what),
        _ => panic!("нет задачи {task}"),
    }
}

// ANCHOR: a
/// `a <bench_N|fixed_N|all> [iters]`: время одной операции и число
/// выделений памяти за вызов замера.
fn task_a(what: &str) {
    use sut::a_bench::*;
    let iters: u32 = arg(3, 100);
    let all = [
        "bench_1", "fixed_1", "bench_2", "fixed_2", "bench_3", "fixed_3", "bench_4", "fixed_4",
        "bench_5", "fixed_5",
    ];
    for name in all.into_iter().filter(|&n| what == "all" || what == n) {
        COUNTING.store(true, Relaxed);
        let before = ALLOCS.load(Relaxed);
        let ns = match name {
            "bench_1" => bench_1(iters),
            "fixed_1" => fixed_1(iters),
            "bench_2" => bench_2(iters),
            "fixed_2" => fixed_2(iters),
            "bench_3" => bench_3(iters),
            "fixed_3" => fixed_3(iters).unwrap_or(f64::NAN),
            "bench_4" => bench_4(iters),
            "fixed_4" => fixed_4(iters),
            "bench_5" => bench_5(iters),
            _ => fixed_5(iters),
        };
        let allocs = ALLOCS.load(Relaxed) - before;
        COUNTING.store(false, Relaxed);
        println!("{name}: {ns:.3}, выделений {allocs}");
    }
}
// ANCHOR_END: a

// ANCHOR: b
/// `b <aos|soa> [n] [samples]`: энергия и шаг по времени для `n` частиц,
/// наносекунды на частицу.
fn task_b(layout: &str) {
    use sut::b_particles::*;
    let n: usize = arg(3, 1 << 20);
    let samples: usize = arg(4, 11);
    let mut ps = cloud(n);
    let (e, energy, step) = match layout {
        "aos" => {
            let energy = bench(samples, 1, || energy_aos(black_box(&ps)));
            let step = bench(samples, 1, || step_aos(black_box(&mut ps), 0.001));
            (energy_aos(&ps), energy, step)
        }
        "soa" => {
            let mut soa = Particles::from_aos(&ps);
            let energy = bench(samples, 1, || black_box(&soa).energy());
            let step = bench(samples, 1, || black_box(&mut soa).step(0.001));
            (soa.energy(), energy, step)
        }
        _ => panic!("раскладка: aos или soa"),
    };
    println!(
        "{layout}: энергия {e}, energy {:.3} нс на частицу, step {:.3} нс",
        energy.median / n as f64,
        step.median / n as f64
    );
}
// ANCHOR_END: b

// ANCHOR: c
/// `c <naive|fast|text> [байт] [n] [k]`: k самых частых n-грамм текста и
/// наносекунды на байт; `text` только готовит текст — это база для
/// подсчёта инструкций.
fn task_c(variant: &str) {
    use sut::c_ngrams::*;
    let len: usize = arg(3, 64 << 20);
    let n: usize = arg(4, 3);
    let k: usize = arg(5, 10);
    let t = text(len);
    let start = Instant::now();
    let top = match variant {
        "naive" => top_naive(&t, n, k),
        "fast" => top_fast(&t, n, k),
        "text" => return,
        _ => panic!("вариант: naive, fast или text"),
    };
    let ns = start.elapsed().as_nanos() as f64 / len as f64;
    for (gram, count) in &top {
        println!("{count:>10} {:?}", String::from_utf8_lossy(gram));
    }
    println!("{variant}: {ns:.3} нс на байт");
}
// ANCHOR_END: c

// ANCHOR: d
/// `d <branchy|branchless> <random|sorted>[-chain] [n] [запросов] [samples]`:
/// поиск в массиве `0, 2, 4, …` длины `n`, наносекунды на запрос. С
/// `-chain` запросы идут цепочкой: младший бит запроса — младший бит суммы
/// прошлых ответов, и следующий поиск не начнётся, пока не кончился этот.
fn task_d(variant: &str) {
    use sut::d_branches::*;
    let arg3 = std::env::args().nth(3).expect("порядок запросов: random или sorted");
    let (order, chain) = arg3.strip_suffix("-chain").map_or((arg3.as_str(), false), |o| (o, true));
    let n: usize = arg(4, 1024);
    let queries: usize = arg(5, 1_000_000);
    let samples: usize = arg(6, 7);
    let v: Vec<u32> = (0..n as u32).map(|i| 2 * i).collect();
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let mut qs: Vec<u32> = (0..queries)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % (2 * n as u64 + 1)) as u32
        })
        .collect();
    match order {
        "random" => {}
        "sorted" => qs.sort_unstable(),
        _ => panic!("порядок: random или sorted, с -chain или без"),
    }
    let f: fn(&[u32], u32) -> usize = match variant {
        "branchy" => lower_bound_branchy,
        "branchless" => lower_bound_branchless,
        _ => panic!("вариант: branchy или branchless"),
    };
    let run = || {
        if chain {
            qs.iter().fold(0usize, |s, &q| s.wrapping_add(f(black_box(&v), q ^ (s as u32 & 1))))
        } else {
            qs.iter().fold(0usize, |s, &q| s.wrapping_add(f(black_box(&v), q)))
        }
    };
    let st = bench(samples, 1, run);
    println!(
        "{variant} {arg3} n={n}: {:.2} нс на запрос, сумма {}",
        st.median / queries as f64,
        run()
    );
}
// ANCHOR_END: d
