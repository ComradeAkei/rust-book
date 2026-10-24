//! Задача C: самые частые n-граммы текста.

use std::collections::HashMap;

// ANCHOR: text
/// `len` байт текста из слов словаря через пробел: распределение n-грамм
/// неравномерное, как у настоящего текста.
pub fn text(len: usize) -> Vec<u8> {
    // ASCII: n-грамма байтов — это n букв, и её можно напечатать.
    const WORDS: [&str; 24] = [
        "the", "of", "and", "to", "a", "in", "is", "station", "sensor", "frame", "value", "time",
        "queue", "bus", "error", "rust", "cache", "clock", "byte", "thread", "core", "data",
        "read", "write",
    ];
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut out = Vec::with_capacity(len + 32);
    while out.len() < len {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        // Слова в начале словаря чаще: квадрат равномерного числа.
        let r = (x >> 40) as f64 / (1u64 << 24) as f64;
        out.extend_from_slice(WORDS[(r * r * WORDS.len() as f64) as usize].as_bytes());
        out.push(b' ');
    }
    out.truncate(len);
    out
}
// ANCHOR_END: text

// ANCHOR: naive
/// `k` самых частых n-грамм байтов: по убыванию числа, при равенстве — по
/// возрастанию самих байтов. Каждая позиция — новый `Vec` и SipHash.
pub fn top_naive(text: &[u8], n: usize, k: usize) -> Vec<(Vec<u8>, u64)> {
    let mut counts: HashMap<Vec<u8>, u64> = HashMap::new();
    for w in text.windows(n) {
        *counts.entry(w.to_vec()).or_insert(0) += 1;
    }
    let mut all: Vec<(Vec<u8>, u64)> = counts.into_iter().collect();
    all.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    all.truncate(k);
    all
}
// ANCHOR_END: naive

// ANCHOR: fast
/// Хеш для ключа `u64`: умножение и сдвиг (как FxHash). Ключи — байты
/// текста, а не данные злоумышленника.
#[derive(Default, Clone, Copy)]
struct MulHasher(u64);

impl core::hash::Hasher for MulHasher {
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

/// То же, что [`top_naive`], для `n` от 1 до 8: n-грамма — число `u64`
/// из её байтов (старший байт — первый, поэтому порядок чисел совпадает с
/// порядком байтов), окно сдвигается на байт без выделений. До двух байт
/// счётчики — просто массив по значению n-граммы (65 536 чисел), длиннее —
/// таблица с дешёвым хешем.
pub fn top_fast(text: &[u8], n: usize, k: usize) -> Vec<(Vec<u8>, u64)> {
    assert!((1..=8).contains(&n), "n от 1 до 8");
    if text.len() < n {
        return Vec::new();
    }
    let mask = if n == 8 { u64::MAX } else { (1u64 << (8 * n)) - 1 };
    let mut key = text[..n - 1].iter().fold(0u64, |a, &b| (a << 8) | u64::from(b));
    let mut found: Vec<(u64, u64)> = if n <= 2 {
        let mut counts = vec![0u64; 1 << (8 * n)];
        for &b in &text[n - 1..] {
            key = ((key << 8) | u64::from(b)) & mask;
            counts[key as usize] += 1;
        }
        counts.iter().enumerate().filter(|&(_, &c)| c > 0).map(|(g, &c)| (g as u64, c)).collect()
    } else {
        let mut counts: HashMap<u64, u64, core::hash::BuildHasherDefault<MulHasher>> =
            HashMap::default();
        for &b in &text[n - 1..] {
            key = ((key << 8) | u64::from(b)) & mask;
            *counts.entry(key).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    };
    let order = |a: &(u64, u64), b: &(u64, u64)| b.1.cmp(&a.1).then(a.0.cmp(&b.0));
    if found.len() > k {
        found.select_nth_unstable_by(k, order);
        found.truncate(k);
    }
    found.sort_unstable_by(order);
    found.into_iter().map(|(g, c)| (g.to_be_bytes()[8 - n..].to_vec(), c)).collect()
}
// ANCHOR_END: fast
