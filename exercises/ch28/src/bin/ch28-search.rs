//! Задача 28.D: замер поиска строки в большом файле.
//!
//! - `ch28-search gen PATH МИБ` — файл из строк псевдослучайного текста,
//!   игла `иголка-в-стоге` — в последней строке.
//! - `ch28-search run PATH [ИГЛА]` — read с буфером 64 КиБ, mmap и mmap с
//!   MADV_HUGEPAGE: время и страничные сбои процесса. Каждый способ —
//!   дважды: с холодным кэшем (страницы файла выгнаны через
//!   `posix_fadvise(DONTNEED)`) и с тёплым.

use ch28_exercises::d_search::{find_mmap, find_read};
use rustix::fs::{Advice, fadvise};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

const NEEDLE: &str = "иголка-в-стоге";

fn generate(path: &str, mib: u64) {
    let mut out = BufWriter::with_capacity(1 << 20, File::create(path).unwrap());
    let mut x = 0x2805_2028_u64;
    let mut written = 0u64;
    let mut line = Vec::with_capacity(80);
    while written < mib << 20 {
        line.clear();
        for _ in 0..79 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            line.push(b"abcdefghijklmnopqrstuvwxyz        "[(x % 34) as usize]);
        }
        line.push(b'\n');
        out.write_all(&line).unwrap();
        written += line.len() as u64;
    }
    writeln!(out, "{NEEDLE}").unwrap();
}

// ANCHOR: measure
/// Сбои страниц процесса: (minor, major).
fn faults() -> (i64, i64) {
    let u = nix::sys::resource::getrusage(nix::sys::resource::UsageWho::RUSAGE_SELF).unwrap();
    (u.minor_page_faults(), u.major_page_faults())
}

fn measure(name: &str, path: &str, cold: bool, search: impl FnOnce(&mut File) -> Option<u64>) {
    let mut file = File::open(path).unwrap();
    let len = file.metadata().unwrap().len();
    if cold {
        // Чистые страницы файла ядро выбрасывает из кэша по просьбе
        // любого, кто может файл открыть; грязные сначала записало бы.
        fadvise(&file, 0, None, Advice::DontNeed).unwrap();
    }
    let (min0, maj0) = faults();
    let t = Instant::now();
    let found = search(&mut file);
    let secs = t.elapsed().as_secs_f64();
    let (min1, maj1) = faults();
    let cache = if cold { "холодный" } else { "тёплый" };
    println!(
        "{name:22} {cache:9} {:7.3} с {:7.0} МБ/с  minor {:7}  major {:6}  {}",
        secs,
        len as f64 / secs / 1e6,
        min1 - min0,
        maj1 - maj0,
        found.map_or("нет".into(), |at| at.to_string()),
    );
}
// ANCHOR_END: measure

fn run(path: &str, needle: &str) {
    let needle = needle.as_bytes();
    // Тёплый замер — сразу после холодного тем же способом: кэш устроен
    // так, как его заполнил этот способ.
    for cold in [true, false] {
        measure("read, буфер 64 КиБ", path, cold, |f| {
            find_read(f, needle, 64 << 10).unwrap()
        });
    }
    for cold in [true, false] {
        // SAFETY: файл замера никто не меняет.
        measure("mmap", path, cold, |f| unsafe { find_mmap(f, needle, false) }.unwrap());
    }
    for cold in [true, false] {
        // SAFETY: как выше.
        measure("mmap + MADV_HUGEPAGE", path, cold, |f| {
            unsafe { find_mmap(f, needle, true) }.unwrap()
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.iter().skip(1).map(String::as_str).collect::<Vec<_>>()[..] {
        ["gen", path, mib] => generate(path, mib.parse().unwrap()),
        ["run", path] => run(path, NEEDLE),
        ["run", path, needle] => run(path, needle),
        _ => {
            eprintln!("использование: ch28-search gen PATH МИБ | run PATH [ИГЛА]");
            std::process::exit(2);
        }
    }
}
