//! Задача D, вопрос к отчёту: поиск без ветвлений и с предвыборкой обоих
//! кандидатов следующего шага — против двух вариантов задачи на трёх
//! размерах массива, запросы по одному и цепочкой. Время — лучший из пяти
//! прогонов по 200 000 случайных запросов.
//!
//! cargo run --release -p ch30-solutions --example d_prefetch

use ch30_solutions::d_branches::{lower_bound_branchless, lower_bound_branchy};
use std::hint::{black_box, select_unpredictable};
use std::time::Instant;

// ANCHOR: prefetch
/// Подсказка процессору: строку кэша с `p` скоро прочтут.
#[inline(always)]
fn prefetch(p: *const u32) {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: SSE есть у любого процессора x86-64. prefetcht0 — подсказка:
    // она ничего не читает в программу и не вызывает сбоя ни на каком
    // адресе; адреса здесь к тому же внутри среза.
    unsafe {
        std::arch::x86_64::_mm_prefetch::<{ std::arch::x86_64::_MM_HINT_T0 }>(p.cast());
    }
    #[cfg(not(target_arch = "x86_64"))]
    let _ = p;
}

/// [`lower_bound_branchless`], который заранее просит в кэш обе середины
/// следующего шага: какая из них понадобится, станет известно, только
/// когда придёт `v[mid]`.
#[inline(never)]
fn lower_bound_prefetch(v: &[u32], x: u32) -> usize {
    let mut size = v.len();
    if size == 0 {
        return 0;
    }
    let mut base = 0;
    while size > 1 {
        let half = size / 2;
        let next = (size - half) / 2;
        prefetch(v.as_ptr().wrapping_add(base + next));
        prefetch(v.as_ptr().wrapping_add(base + half + next));
        let mid = base + half;
        base = select_unpredictable(v[mid] < x, mid, base);
        size -= half;
    }
    base + usize::from(v[base] < x)
}
// ANCHOR_END: prefetch

fn main() {
    if cfg!(not(target_arch = "x86_64")) {
        println!("предвыборка есть только на x86-64: третий вариант — тот же branchless");
    }
    type Search = fn(&[u32], u32) -> usize;
    let variants: [(&str, Search); 3] = [
        ("branchy", lower_bound_branchy),
        ("branchless", lower_bound_branchless),
        ("prefetch", lower_bound_prefetch),
    ];
    for n in [1usize << 10, 1 << 16, 1 << 24] {
        let v: Vec<u32> = (0..n as u32).map(|i| 2 * i).collect();
        let mut x = 0x2545_F491_4F6C_DD1Du64;
        let qs: Vec<u32> = (0..200_000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x % (2 * n as u64 + 1)) as u32
            })
            .collect();
        for (name, f) in variants {
            // Один прогон: по одному или цепочкой, как в ch30-task.
            let run = |chain: bool| {
                let start = Instant::now();
                let sum = qs.iter().fold(0usize, |s, &q| {
                    let q = if chain { q ^ (s as u32 & 1) } else { q };
                    s.wrapping_add(f(black_box(&v), q))
                });
                (start.elapsed().as_nanos() as f64 / qs.len() as f64, sum)
            };
            let best = |chain: bool| (0..5).map(|_| run(chain).0).fold(f64::INFINITY, f64::min);
            let (one, chain) = (best(false), best(true));
            // Суммы ответов у трёх вариантов должны совпасть.
            let (sums, chained) = (run(false).1, run(true).1);
            println!(
                "n = {n:>8} {name:>10}: по одному {one:6.1} нс, цепочкой {chain:6.1} нс \
                 (суммы {sums}, {chained})"
            );
        }
    }
}
