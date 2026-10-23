//! Задача D: цена пары `Arc::clone` + `drop` при 1, 2, 4 … N потоках —
//! на общем счётчике, на своём у каждого потока (как выделил
//! распределитель и выровненном по кэш-линии) и без клонов вовсе.
//!
//! Столбец «свой» снимается в отдельном процессе на свежей куче: там
//! распределитель кладёт блоки `Arc` подряд, по два на линию, как в
//! программе, которая выделила их при старте. В процессе, где до этого
//! работали потоки, он отдаёт освобождённые блоки вразброс, и соседство
//! линий — дело случая. Рядом с временем печатаются номера линий
//! счётчиков, считая от первой.
//!
//! Запуск: `cargo run --release -p ch14-solutions --example arc_threads [итераций]`.
//! Время меняется от машины к машине и от запуска к запуску.
use ch14_solutions::d_arc::{borrowed, lines, private_lines, private_padded, shared};
use std::process::Command;

/// Лучший из трёх замеров.
fn best(f: fn(usize, usize) -> f64, threads: usize, iters: usize) -> f64 {
    (0..3).map(|_| f(threads, iters)).fold(f64::INFINITY, f64::min)
}

/// «Свой» в свежем процессе: время и линии счётчиков от первой. Лучший
/// из трёх процессов: повтор в том же процессе получил бы другие блоки.
fn private_fresh(threads: usize, iters: usize) -> (f64, Vec<usize>) {
    let run = || {
        let out = Command::new(std::env::current_exe().unwrap())
            .args(["--private", &threads.to_string(), &iters.to_string()])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let text = String::from_utf8(out.stdout).unwrap();
        let mut words = text.split_whitespace().map(|w| w.parse::<f64>().unwrap());
        let ns = words.next().unwrap();
        (ns, words.map(|l| l as usize).collect::<Vec<_>>())
    };
    (0..3).map(|_| run()).min_by(|a, b| a.0.total_cmp(&b.0)).unwrap()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--private") {
        let (threads, iters) = (args[1].parse().unwrap(), args[2].parse().unwrap());
        // Один замер: только первый идёт на свежей куче.
        let (ns, lines) = private_lines(threads, iters);
        let first = lines.iter().min().unwrap();
        print!("{ns}");
        for l in &lines {
            print!(" {}", l - first);
        }
        println!();
        return;
    }
    let iters = args.first().map_or(5_000_000, |s| s.parse().unwrap());
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut counts: Vec<usize> = (0..).map(|i| 1 << i).take_while(|&n| n <= cores).collect();
    if counts.last() != Some(&cores) {
        counts.push(cores);
    }
    println!("ядер: {cores}; нс на пару clone + drop (на чтение для «без клонов»)");
    println!("потоков   общий   свой   свой выровненный   без клонов   линии своих");
    for n in counts {
        let (own, own_lines) = private_fresh(n, iters);
        println!(
            "{n:7} {:7.1} {:6.1} {:18.1} {:12.2}   {own_lines:?}",
            best(shared, n, iters),
            own,
            best(private_padded, n, iters),
            best(borrowed, n, iters)
        );
    }
    let first = |v: Vec<usize>| {
        let m = *v.iter().min().unwrap();
        v.into_iter().map(|l| l - m).collect::<Vec<_>>()
    };
    println!("линии выровненных счётчиков: {:?}", first(lines(cores, true)));
}
