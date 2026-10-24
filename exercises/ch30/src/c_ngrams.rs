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
/// То же, что [`top_naive`], для `n` от 1 до 8, но минимум в десять раз
/// быстрее.
pub fn top_fast(text: &[u8], n: usize, k: usize) -> Vec<(Vec<u8>, u64)> {
    let _ = (text, n, k);
    todo!("30.C: top_fast")
}
// ANCHOR_END: fast
