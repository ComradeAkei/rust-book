//! Задача B: распределитель-счётчик на решениях прошлых глав. Сколько
//! раз они просят память, сколько байт запрашивают и каков пик.
//!
//! Запуск: `cargo run --release -p ch15-solutions --example report`.
use ch11_solutions::c_dispatch::{Eight, build_dyn, build_enum, kinds};
use ch12_solutions::c_parser::parse_csq;
use ch13_solutions::d_vector::CHAINS;
use ch14_solutions::c_list::{ArenaList, RcList};
use ch15_solutions::b_count::{Counting, Stats, measure};
use std::hint::black_box;

// ANCHOR: install
#[global_allocator]
static COUNTING: Counting = Counting;
// ANCHOR_END: install

fn line(what: &str, s: Stats) {
    println!(
        "  {what:68} alloc {:4}, realloc {:2}, запрошено {:7} Б, пик {:6} Б",
        s.allocs, s.reallocs, s.requested, s.peak
    );
}

fn main() {
    println!("13.D: collect по вектору из 1000 чисел u32");
    for (text, f) in CHAINS {
        let v: Vec<u32> = (0..1000).collect();
        let ((), s) = measure(|| f(v));
        line(text, s);
    }

    println!("11.C: 1000 операций восьми типов");
    let k = kinds(8, 1000, false);
    let (d, s) = measure(|| build_dyn::<Eight>(&k));
    line("Vec<Box<dyn Op>>", s);
    drop(d);
    let (e, s) = measure(|| build_enum::<Eight>(&k));
    line("Vec<Any8>", s);
    drop(e);

    println!("12.C: ответ модема разобран комбинаторами");
    let (r, s) = measure(|| parse_csq(black_box("\r\n+CSQ: 23,99\r\n\r\nOK\r\n")));
    assert!(r.is_some());
    line("parse_csq", s);

    println!("14.C: двусвязный список из 1000 узлов");
    let ((), s) = measure(|| {
        let mut l = RcList::new();
        for i in 0..1000u64 {
            black_box(l.push_back(i));
        }
    });
    line("RcList: постройка и уничтожение", s);
    let ((), s) = measure(|| {
        let mut l = ArenaList::new();
        for i in 0..1000u64 {
            black_box(l.push_back(i));
        }
    });
    line("ArenaList: постройка и уничтожение", s);
}
