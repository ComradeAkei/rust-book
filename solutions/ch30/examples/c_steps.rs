//! Задача C по шагам: что даёт каждое изменение наивного подсчёта
//! n-грамм по отдельности. Каждый шаг — своя функция, чтобы её было видно
//! в профиле perf.
//!
//! cargo run --release -p ch30-solutions --example c_steps -- [байт] [n] [шаг]

use ch30_solutions::c_ngrams::{text, top_fast, top_naive};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::time::Instant;

// ANCHOR: hashers
/// Хеш байтов по одному, как FxHash из rustc: сдвиг, xor, умножение.
#[derive(Default, Clone, Copy)]
struct FxLike(u64);

impl Hasher for FxLike {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(b)).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// Хеш одного `u64` — одно умножение, как в `top_fast`.
#[derive(Default, Clone, Copy)]
struct Mul(u64);

impl Hasher for Mul {
    fn write(&mut self, _: &[u8]) {
        unreachable!("ключ — только u64")
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = n.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(26);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}
// ANCHOR_END: hashers

// ANCHOR: steps
/// Ключ — срез текста: выделений нет, хеш — SipHash.
#[inline(never)]
fn slice_sip(t: &[u8], n: usize) -> usize {
    let mut m: HashMap<&[u8], u64> = HashMap::new();
    for w in t.windows(n) {
        *m.entry(w).or_insert(0) += 1;
    }
    m.len()
}

/// Ключ — `Vec`, как в наивном подсчёте, но хеш дешёвый.
#[inline(never)]
fn vec_fx(t: &[u8], n: usize) -> usize {
    let mut m: HashMap<Vec<u8>, u64, BuildHasherDefault<FxLike>> = HashMap::default();
    for w in t.windows(n) {
        *m.entry(w.to_vec()).or_insert(0) += 1;
    }
    m.len()
}

/// n-грамма — число `u64`, окно сдвигается на байт; хеш — SipHash.
#[inline(never)]
fn u64_sip(t: &[u8], n: usize) -> usize {
    let mask = (1u64 << (8 * n)) - 1;
    let mut key = t[..n - 1].iter().fold(0u64, |a, &b| (a << 8) | u64::from(b));
    let mut m: HashMap<u64, u64> = HashMap::new();
    for &b in &t[n - 1..] {
        key = ((key << 8) | u64::from(b)) & mask;
        *m.entry(key).or_insert(0) += 1;
    }
    m.len()
}

/// Ключ — `u64`, хеш — одно умножение.
#[inline(never)]
fn u64_mul(t: &[u8], n: usize) -> usize {
    let mask = (1u64 << (8 * n)) - 1;
    let mut key = t[..n - 1].iter().fold(0u64, |a, &b| (a << 8) | u64::from(b));
    let mut m: HashMap<u64, u64, BuildHasherDefault<Mul>> = HashMap::default();
    for &b in &t[n - 1..] {
        key = ((key << 8) | u64::from(b)) & mask;
        *m.entry(key).or_insert(0) += 1;
    }
    m.len()
}
// ANCHOR_END: steps

fn main() {
    let arg = |i: usize| std::env::args().nth(i);
    let len: usize = arg(1).map_or(64 << 20, |s| s.parse().expect("байт: число"));
    let n: usize = arg(2).map_or(3, |s| s.parse().expect("n: число"));
    assert!((1..8).contains(&n), "n от 1 до 7");
    let only = arg(3);
    let t = text(len);
    // naive и fast возвращают 10 частых n-грамм, промежуточные шаги — число
    // разных n-грамм: сортировки в них нет, только подсчёт. Разных n-грамм
    // всего сотни, и сортировка их ничего не стоит.
    let steps: [(&str, &dyn Fn() -> usize); 6] = [
        ("naive", &|| top_naive(&t, n, 10).len()),
        ("slice_sip", &|| slice_sip(&t, n)),
        ("vec_fx", &|| vec_fx(&t, n)),
        ("u64_sip", &|| u64_sip(&t, n)),
        ("u64_mul", &|| u64_mul(&t, n)),
        ("fast", &|| top_fast(&t, n, 10).len()),
    ];
    for (name, step) in steps {
        if only.as_deref().is_some_and(|o| o != name) {
            continue;
        }
        let start = Instant::now();
        let found = step();
        let ns = start.elapsed().as_nanos() as f64 / len as f64;
        println!("{name:>9}: {ns:6.2} нс на байт ({found})");
    }
}
