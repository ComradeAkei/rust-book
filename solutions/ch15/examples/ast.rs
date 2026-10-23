//! Задача C: одно и то же дерево формулы из `Box` и из `Bump`. Печатает
//! время разбора, вычисления и уничтожения на узел, число обращений к
//! распределителю и пик памяти; системные вызовы — через strace:
//!
//! ```console
//! $ cargo build --release -p ch15-solutions --example ast
//! $ strace -c -e trace=brk,mmap,munmap,mremap target/release/examples/ast box
//! $ strace -c -e trace=brk,mmap,munmap,mremap target/release/examples/ast bump
//! ```
//!
//! Без аргумента — оба дерева, каждое в своём процессе.
use ch15_solutions::b_count::{Counting, measure};
use ch15_solutions::c_bump::{Boxes, Bump, InBump, gen_source, parse};
use std::hint::black_box;
use std::time::Instant;

#[global_allocator]
static COUNTING: Counting = Counting;

fn vars(name: &str) -> i64 {
    name.len() as i64
}

/// Пик резидентной памяти процесса из /proc: сколько страниц на деле
/// заняли блоки вместе с заголовками и выравниванием распределителя.
fn peak_rss_mib() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let kb = status
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.trim().trim_end_matches(" kB").parse::<f64>().ok());
    kb.unwrap_or(0.0) / 1024.0
}

fn ns(t: Instant, n: usize) -> f64 {
    t.elapsed().as_secs_f64() * 1e9 / n as f64
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    let n: usize = std::env::args().nth(2).map_or(1 << 20, |s| s.parse().unwrap());
    let src = gen_source(n, 42);
    match which.as_str() {
        "box" => {
            let t = Instant::now();
            let (tree, s) = measure(|| parse(&src, Boxes).unwrap());
            let parse_ns = ns(t, n);
            let t = Instant::now();
            black_box(black_box(&tree).eval(&vars));
            let eval_ns = ns(t, n);
            let t = Instant::now();
            drop(tree);
            let drop_ns = ns(t, n);
            println!(
                "Box:  разбор {parse_ns:5.1} нс, вычисление {eval_ns:4.1} нс, уничтожение {drop_ns:4.1} нс на узел; \
                 alloc {}, пик {:.1} МиБ, RSS {:.1} МиБ",
                s.allocs,
                s.peak as f64 / (1 << 20) as f64,
                peak_rss_mib()
            );
        }
        "bump" => {
            let bump = Bump::new();
            let t = Instant::now();
            let (tree, s) = measure(|| parse(&src, InBump(&bump)).unwrap());
            let parse_ns = ns(t, n);
            let t = Instant::now();
            black_box(black_box(tree).eval(&vars));
            let eval_ns = ns(t, n);
            let (chunks, bytes) = bump.footprint();
            let t = Instant::now();
            drop(bump);
            let drop_ns = ns(t, n);
            println!(
                "Bump: разбор {parse_ns:5.1} нс, вычисление {eval_ns:4.1} нс, уничтожение {drop_ns:4.1} нс на узел; \
                 alloc {}, пик {:.1} МиБ, кусков {chunks} ({:.1} МиБ), RSS {:.1} МиБ",
                s.allocs,
                s.peak as f64 / (1 << 20) as f64,
                bytes as f64 / (1 << 20) as f64,
                peak_rss_mib()
            );
        }
        _ => {
            for w in ["box", "bump"] {
                let me = std::env::current_exe().unwrap();
                let st = std::process::Command::new(me).args([w, &n.to_string()]).status();
                assert!(st.unwrap().success());
            }
        }
    }
}
