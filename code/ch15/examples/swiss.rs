//! Раздел 15.3: управляющие байты настоящей `HashMap` из `std`. Пример
//! запоминает последний блок, который выдал распределитель, — это и есть
//! таблица, — и печатает её управляющий массив: `ff` — пусто, `80` —
//! удалено, `00`–`7f` — занято, и тогда это старшие семь бит хеша ключа.
//!
//! Раскладка блока — наблюдение для rustc 1.97 (внутри `std` живёт
//! hashbrown): `корзины × размер пары`, затем управляющие байты, по одному
//! на корзину, и ещё 16 — копия первых, чтобы группу из 16 байт можно было
//! читать с любой позиции.
//!
//! Запуск: `cargo run --release -p ch15 --example swiss`.
use ch15::maps::{Kr, colliding_name, h1_h2};
use rustc_hash::FxBuildHasher;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// Распределитель, который помнит адрес и размер последнего блока.
struct Last;

static PTR: AtomicUsize = AtomicUsize::new(0);
static SIZE: AtomicUsize = AtomicUsize::new(0);

// SAFETY: все вызовы передаются системному распределителю как есть.
unsafe impl GlobalAlloc for Last {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: контракт alloc тот же, что у вызывающего.
        let p = unsafe { System.alloc(layout) };
        PTR.store(p as usize, Relaxed);
        SIZE.store(layout.size(), Relaxed);
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static LAST: Last = Last;

// ANCHOR: ctrl
/// Управляющие байты таблицы, которую `std` только что выделила: блок
/// начинается с корзин, за ними `buckets + 16` байт управления.
fn ctrl(block: usize, buckets: usize, pair: usize) -> Vec<u8> {
    let ctrl = block + (buckets * pair).next_multiple_of(16);
    // SAFETY: блок жив (таблица не уничтожена), управляющие байты
    // инициализированы при создании таблицы; только чтение.
    (0..buckets + 16).map(|i| unsafe { *((ctrl + i) as *const u8) }).collect()
}
// ANCHOR_END: ctrl

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

fn main() {
    // Детерминированный хешер, чтобы вывод не менялся от запуска к запуску.
    let mut m: HashMap<u64, u64, FxBuildHasher> = HashMap::default();
    for k in 0..15 {
        m.insert(k * 100, k);
    }
    let (block, size) = (PTR.load(Relaxed), SIZE.load(Relaxed));
    let buckets = (m.capacity() + 1).next_power_of_two();
    println!(
        "HashMap<u64, u64>, 15 ключей: ёмкость {}, корзин {buckets}, блок {size} байт = {buckets} × 16 + {buckets} + 16",
        m.capacity()
    );
    let c = ctrl(block, buckets, 16);
    println!("управление: {}", hex(&c[..buckets]));
    println!("хвост:      {} (копия первых 16)", hex(&c[buckets..]));
    assert_eq!(c[..16], c[buckets..]);
    for k in [300, 700, 1100] {
        let (h1, h2) = h1_h2(m.hasher().hash_one(k), buckets);
        let at = (h1..).map(|i| i % buckets).find(|&i| c[i] == h2).unwrap();
        println!("ключ {k:4}: h1 = {h1:2}, h2 = {h2:02x}, байт {h2:02x} в позиции {at}");
    }

    // Двадцать имён с одним хешем K&R: одна серия занятых корзин подряд.
    // Таблица выделена сразу: дальше блоки выделяют только строки-ключи.
    let mut kr: HashMap<String, u32, Kr> = HashMap::with_capacity_and_hasher(20, Kr::default());
    let (block, buckets) = (PTR.load(Relaxed), (kr.capacity() + 1).next_power_of_two());
    for i in 0..20 {
        kr.insert(colliding_name(i, 5), i);
    }
    let pair = size_of::<(String, u32)>();
    let (h1, h2) = h1_h2(kr.hasher().hash_one("AaAaAaAaAa"), buckets);
    println!("\n20 имён из Aa/BB, хеш K&R: корзин {buckets}, у всех h1 = {h1}, h2 = {h2:02x}");
    println!("управление: {}", hex(&ctrl(block, buckets, pair)[..buckets]));
    kr.remove(&colliding_name(5, 5));
    let after = ctrl(block, buckets, pair);
    println!("без {}: {}", colliding_name(5, 5), hex(&after[..buckets]));
    let found = (0..20).filter(|&i| i != 5 && kr.contains_key(&colliding_name(i, 5))).count();
    println!("удалено помечено 80, остальные 19 находятся: {}", found == 19);
}
