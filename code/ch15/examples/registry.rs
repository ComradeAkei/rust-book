//! Введение главы 15: три сценария `table.c` на `HashMap`. Указатель на
//! запись не переживает вставку — вместо него ключ и повторный поиск через
//! `entry`; удаление не теряет соседей; на именах из «Aa» и «BB» таблица с
//! хешем K&R сравнивает каждый новый ключ со всеми прежними, а с SipHash —
//! почти ни с одним.
//!
//! Запуск: `cargo run --release -p ch15 --example registry`. С аргументом
//! `order` пример печатает порядок обхода двух таблиц с одними ключами.
use ch15::maps::{Kr, colliding_name};
use std::cell::Cell;
use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::time::Instant;

#[derive(Default)]
struct Entry {
    count: u32,
    last: i32,
}

thread_local! {
    static EQ: Cell<u64> = const { Cell::new(0) };
}

// ANCHOR: name
/// Имя датчика, которое считает свои сравнения на равенство: так видно,
/// сколько ключей таблица перебрала, прежде чем найти место.
#[derive(Clone)]
struct Name(String);

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        EQ.with(|c| c.set(c.get() + 1));
        self.0 == other.0
    }
}

impl Eq for Name {}

impl Hash for Name {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.0.hash(h)
    }
}
// ANCHOR_END: name

// ANCHOR: dangle
fn scenario_dangle() {
    let mut t: HashMap<String, Entry> = HashMap::new();
    t.insert("adc0".into(), Entry::default());
    for i in 0..16 {
        t.insert(format!("i2c{i}"), Entry::default());
    }
    // Вместо указателя, сохранённого до вставок, — поиск по ключу сейчас.
    let adc = t.entry("adc0".into()).or_default();
    adc.count += 1;
    adc.last = 1250;
    println!("adc0: измерений {} (ожидалось 1)", t["adc0"].count);
}
// ANCHOR_END: dangle

fn scenario_delete() {
    let mut t: HashMap<&str, u32, Kr> = HashMap::default();
    t.insert("Aa", 1);
    t.insert("BB", 2);
    t.remove("Aa");
    println!("BB: {}", if t.contains_key("BB") { "найден" } else { "не найден" });
}

// ANCHOR: fill
fn fill<S: BuildHasher + Default>(n: u32, k: u32) -> (f64, f64) {
    let mut t: HashMap<Name, u32, S> = HashMap::default();
    let names: Vec<Name> = (0..n)
        .map(|i| Name(if k > 0 { colliding_name(i, k) } else { format!("sensor{i:05}") }))
        .collect();
    EQ.with(|c| c.set(0));
    let start = Instant::now();
    for name in names {
        t.insert(name, 0);
    }
    let ms = start.elapsed().as_secs_f64() * 1e3;
    (EQ.with(Cell::get) as f64 / n as f64, ms)
}
// ANCHOR_END: fill

fn scenario_collide() {
    let k = 13;
    let n = 1 << k;
    for (hasher, normal, colliding) in [
        ("SipHash", fill::<RandomState>(n, 0), fill::<RandomState>(n, k)),
        ("K&R    ", fill::<Kr>(n, 0), fill::<Kr>(n, k)),
    ] {
        println!(
            "{hasher}, обычные имена:  {n} штук, {:.1} сравнений на вставку, {:.2} мс",
            normal.0, normal.1
        );
        println!(
            "{hasher}, имена из Aa/BB: {n} штук, {:.1} сравнений на вставку, {:.2} мс",
            colliding.0, colliding.1
        );
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("order") {
        // Две таблицы с одинаковыми ключами в одном процессе.
        println!("{:?}", ch15::maps::iteration_order(16));
        println!("{:?}", ch15::maps::iteration_order(16));
        return;
    }
    scenario_dangle();
    scenario_delete();
    scenario_collide();
}
