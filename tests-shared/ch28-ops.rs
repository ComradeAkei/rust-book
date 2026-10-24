//! Задача 28.A: шесть операций подряд, разделённых вызовом `getppid`.
//! Тест запускает программу под `strace -f` и делит журнал по меткам:
//! вызовы между меткой k и k+1 — это операция k. `getppid` ничего не
//! меняет и в std больше нигде не встречается, поэтому годится в метки.
//!
//! Программа общая для заготовок и решений: её объявляют `[[bin]]` обоих
//! крейтов.

use std::hint::black_box;
use std::io::Read;
use std::os::unix::process::parent_id;
use std::time::Instant;

// ANCHOR: ops
fn mark() {
    black_box(parent_id());
}

fn main() {
    mark();
    println!("строка");
    mark();
    let mut file = std::fs::File::open("/etc/passwd").unwrap();
    mark();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    mark();
    let out = std::process::Command::new("true").output().unwrap();
    mark();
    std::thread::spawn(|| black_box(1)).join().unwrap();
    mark();
    let now = Instant::now();
    mark();
    black_box((file, text, out, now));
}
// ANCHOR_END: ops
