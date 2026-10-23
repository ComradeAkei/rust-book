// Задача 10.B: время qsort из libc и sort_unstable_by на одних данных.
//
// Файл не является крейтом: его подключают через `include!` примеры
// `exercises/ch10/examples/b_bench.rs` и `solutions/ch10/examples/b_bench.rs`,
// заранее объявив `use <крейт> as sut;`.
//
// Запуск: cargo run --release -p ch10-solutions --example b_bench [n]
// По умолчанию n = 10 000 000. Каждая сортировка идёт пять раз на копии
// одних и тех же данных, печатается лучшее время.

use std::time::Instant;
use sut::b_sort::{sort_libc, sort_rust};

fn main() {
    let n: usize = std::env::args().nth(1).map_or(10_000_000, |s| s.parse().unwrap());
    // xorshift32: одни и те же данные при каждом запуске.
    let mut x = 12345u32;
    let data: Vec<u32> = (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x
        })
        .collect();
    // Третий вариант отделяет алгоритм от встраивания: та же сортировка
    // из core, но сравнение — косвенный вызов через указатель, который
    // black_box прячет от оптимизатора.
    fn sort_ptr(v: &mut [u32]) {
        let cmp: fn(&u32, &u32) -> std::cmp::Ordering = std::hint::black_box(u32::cmp);
        v.sort_unstable_by(cmp);
    }
    type Sort = fn(&mut [u32]);
    let sorts: [(&str, Sort); 3] =
        [("qsort", sort_libc), ("sort_unstable_by", sort_rust), ("… через указатель", sort_ptr)];
    for (name, sort) in sorts {
        let mut best = f64::INFINITY;
        for _ in 0..5 {
            let mut v = data.clone();
            let t = Instant::now();
            sort(&mut v);
            best = best.min(t.elapsed().as_secs_f64());
            assert!(v.is_sorted(), "{name}: порядок нарушен");
        }
        println!("{name:>18}: {:8.1} мс на {n} элементах", best * 1e3);
    }
}
