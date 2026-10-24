//! Разделы 15.3–15.5: замеры. Что печатается — время на элемент или на
//! операцию, лучшее из пяти прогонов.
//!
//! - `locality` — сумма 4 Ми чисел из `Vec<u64>`, из `Vec<Box<u64>>`
//!   (блоки по порядку и вперемешку) и из `LinkedList<u64>` (узлы подряд и
//!   вперемешку с чужими блоками).
//! - `hash` — вставка и поиск 4 Ki и 1 Mi ключей `u64` и строк с
//!   SipHash-1-3, FxHash и foldhash.
//! - `lookup` — поиск по отсортированному `Vec`, `BTreeMap` и `HashMap`
//!   на 1 Ki, 64 Ki и 1 Mi ключей.
//!
//! Запуск: `cargo run --release -p ch15 --example bench [locality|hash|lookup]`.
use ch15::locality::{scattered_list, shuffled_boxes, sum_boxes, sum_list, sum_vec};
use std::collections::{BTreeMap, HashMap, LinkedList};
use std::hash::BuildHasher;
use std::hint::black_box;
use std::time::Instant;

/// Лучшее из пяти время одного прогона `f`, в наносекундах на `ops`.
fn best(ops: usize, mut f: impl FnMut()) -> f64 {
    (0..5)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1e9 / ops as f64
        })
        .fold(f64::INFINITY, f64::min)
}

fn xorshift(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

// ANCHOR: locality
fn locality() {
    let n = 1 << 22;
    let v: Vec<u64> = (0..n as u64).collect();
    println!(
        "Vec<u64>:                     {:5.2} нс",
        best(n, || {
            black_box(sum_vec(black_box(&v)));
        })
    );
    let boxes: Vec<Box<u64>> = (0..n as u64).map(Box::new).collect();
    println!(
        "Vec<Box<u64>>, по порядку:    {:5.2} нс",
        best(n, || {
            black_box(sum_boxes(black_box(&boxes)));
        })
    );
    drop(boxes);
    let shuffled = shuffled_boxes(n, 0x9E37_79B9_7F4A_7C15);
    println!(
        "Vec<Box<u64>>, вперемешку:    {:5.2} нс",
        best(n, || {
            black_box(sum_boxes(black_box(&shuffled)));
        })
    );
    drop(shuffled);
    let list: LinkedList<u64> = v.iter().copied().collect();
    println!(
        "LinkedList<u64>, подряд:      {:5.2} нс",
        best(n, || {
            black_box(sum_list(black_box(&list)));
        })
    );
    drop(list);
    let list = scattered_list(n, 7);
    println!(
        "LinkedList<u64>, вперемешку:  {:5.2} нс",
        best(n, || {
            black_box(sum_list(black_box(&list)));
        })
    );
}
// ANCHOR_END: locality

fn hash_u64<S: BuildHasher + Default>(name: &str, keys: &[u64]) {
    let mut m: HashMap<u64, u64, S> = HashMap::default();
    let insert = best(keys.len(), || {
        m = HashMap::default();
        for &k in keys {
            m.insert(k, k);
        }
    });
    let get = best(keys.len(), || {
        let mut s = 0;
        for k in keys {
            s += m[k];
        }
        black_box(s);
    });
    println!("{name:9} u64:    вставка {insert:5.1} нс, поиск {get:5.1} нс");
}

fn hash_str<S: BuildHasher + Default>(name: &str, keys: &[String]) {
    let mut m: HashMap<&str, usize, S> = HashMap::default();
    let insert = best(keys.len(), || {
        m = HashMap::default();
        for (i, k) in keys.iter().enumerate() {
            m.insert(k, i);
        }
    });
    let get = best(keys.len(), || {
        let mut s = 0;
        for k in keys {
            s += m[k.as_str()];
        }
        black_box(s);
    });
    println!("{name:9} &str:   вставка {insert:5.1} нс, поиск {get:5.1} нс");
}

// ANCHOR: hash
fn hash() {
    // 4 Ki ключей — таблица в кэше, видна цена самого хеша; 1 Mi — таблица
    // в памяти, и к ней добавляются промахи.
    for n in [1 << 12, 1 << 20] {
        println!("{n} ключей:");
        let mut seed = 1;
        let keys: Vec<u64> = (0..n).map(|_| xorshift(&mut seed)).collect();
        let names: Vec<String> = keys.iter().map(|k| format!("sensor-{k:x}")).collect();
        hash_u64::<std::collections::hash_map::RandomState>("SipHash", &keys);
        hash_u64::<rustc_hash::FxBuildHasher>("FxHash", &keys);
        hash_u64::<foldhash::fast::RandomState>("foldhash", &keys);
        hash_str::<std::collections::hash_map::RandomState>("SipHash", &names);
        hash_str::<rustc_hash::FxBuildHasher>("FxHash", &names);
        hash_str::<foldhash::fast::RandomState>("foldhash", &names);
    }
}
// ANCHOR_END: hash

// ANCHOR: lookup
fn lookup() {
    for n in [1 << 10, 1 << 16, 1 << 20] {
        let mut seed = 3;
        let mut sorted: Vec<u64> = (0..n).map(|_| xorshift(&mut seed)).collect();
        sorted.sort_unstable();
        let tree: BTreeMap<u64, ()> = sorted.iter().map(|&k| (k, ())).collect();
        let map: HashMap<u64, ()> = sorted.iter().map(|&k| (k, ())).collect();
        let probes: Vec<u64> =
            (0..1 << 20).map(|i| sorted[xorshift(&mut seed) as usize % n] + (i & 1)).collect();
        let vec = best(probes.len(), || {
            black_box(probes.iter().filter(|k| sorted.binary_search(k).is_ok()).count());
        });
        let bt = best(probes.len(), || {
            black_box(probes.iter().filter(|k| tree.contains_key(k)).count());
        });
        let hm = best(probes.len(), || {
            black_box(probes.iter().filter(|k| map.contains_key(k)).count());
        });
        println!(
            "{n:8} ключей: Vec + binary_search {vec:5.1} нс, BTreeMap {bt:5.1} нс, HashMap {hm:5.1} нс"
        );
    }
}
// ANCHOR_END: lookup

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("locality") => locality(),
        Some("hash") => hash(),
        Some("lookup") => lookup(),
        _ => {
            locality();
            hash();
            lookup();
        }
    }
}
