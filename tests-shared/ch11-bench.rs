// Задача 11.C: перечисление, dyn и обобщения на 2, 8 и 64 типах, элементы
// по порядку типов и вперемешку.
//
// Файл не является крейтом: его подключают через `include!` примеры
// `exercises/ch11/examples/c_bench.rs` и `solutions/ch11/examples/c_bench.rs`,
// заранее объявив `use <крейт> as sut;`.
//
// Запуск: cargo run --release -p ch11-solutions --example c_bench [n [повторов [замер]]]
// По умолчанию n = 65 536 элементов и 200 повторов. Каждый замер идёт пять
// раз, печатается лучшее время на элемент. Третий аргумент оставляет один
// замер, например `8/shuffled/dyn` (типов, `sorted` или `shuffled`, `dyn`,
// `enum` или `groups`): так его можно прогнать под valgrind или perf.

use std::hint::black_box;
use std::time::Instant;
use sut::c_dispatch::{Eight, Family, SixtyFour, Two, build_dyn, build_enum, kinds, run};

/// Лучшее из пяти время одного прохода в наносекундах на элемент. Не
/// встраивается, чтобы `valgrind --toggle-collect='*best*'` считал только
/// замеры, без построения массивов.
#[inline(never)]
fn best(n: usize, reps: u32, mut pass: impl FnMut(u32) -> u32) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..5 {
        let t = Instant::now();
        let mut acc = 0u32;
        for r in 0..reps {
            acc = acc.wrapping_add(pass(black_box(r)));
        }
        black_box(acc);
        best = best.min(t.elapsed().as_secs_f64() * 1e9 / (n as f64 * f64::from(reps)));
    }
    best
}

fn measure<F: Family>(n: usize, reps: u32, only: Option<&str>) {
    for (order, sorted) in [("по порядку", true), ("вперемешку", false)] {
        let key =
            |kind: &str| format!("{}/{}/{kind}", F::N, if sorted { "sorted" } else { "shuffled" });
        let want = |kind: &str| only.is_none_or(|o| o == key(kind));
        if !["dyn", "enum", "groups"].into_iter().any(want) {
            continue;
        }
        let k = kinds(F::N, n, sorted);
        let objects = build_dyn::<F>(&k);
        let variants = build_enum::<F>(&k);
        let groups = F::groups(&k);
        // Одна и та же сумма тремя способами: порядок сложения по модулю
        // 2³² на результат не влияет.
        assert_eq!(run(&objects, 7), run(&variants, 7));
        assert_eq!(run(&objects, 7), F::run_groups(&groups, 7));
        let d = want("dyn").then(|| best(n, reps, |x| run(&objects, x)));
        let e = want("enum").then(|| best(n, reps, |x| run(&variants, x)));
        let stat = want("groups").then(|| best(n, reps, |x| F::run_groups(&groups, x)));
        let cell =
            |t: Option<f64>, w: usize| t.map_or(format!("{:>w$}", "—"), |t| format!("{t:w$.2}"));
        println!("{:>5}  {order:<11} {} {} {}", F::N, cell(d, 6), cell(e, 6), cell(stat, 10));
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let n: usize = args.next().map_or(65_536, |s| s.parse().unwrap());
    let reps: u32 = args.next().map_or(200, |s| s.parse().unwrap());
    let only = args.next();
    println!("типов  порядок        dyn   enum  обобщения   (нс на элемент, {n} элементов)");
    measure::<Two>(n, reps, only.as_deref());
    measure::<Eight>(n, reps, only.as_deref());
    measure::<SixtyFour>(n, reps, only.as_deref());
}
