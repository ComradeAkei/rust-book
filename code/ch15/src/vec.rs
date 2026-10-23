//! Раздел 15.1: `Vec` — указатель, ёмкость и длина. Как растёт буфер,
//! что обещают `with_capacity`, `reserve` и `shrink_to_fit`, почему ссылка
//! на элемент не переживает `push` и как спросить память без паники.
//!
//! Конкретные ёмкости после роста — наблюдение для rustc 1.97: стратегию
//! роста `Vec` не обещает. Гарантии — в документации `Vec`, раздел
//! «Guarantees».
//!
//! Ссылка на элемент и `push` вместе не живут:
//!
//! ```compile_fail,E0502
//! let mut v = vec![10, 20, 30];
//! let first = &v[0];
//! v.push(40); // буфер может переехать
//! println!("{first}");
//! ```

use alloc::collections::TryReserveError;
use alloc::vec::Vec;

// ANCHOR: parts
/// Три слова `Vec`: где лежат элементы, сколько места выделено и сколько
/// из него занято. Порядок полей в памяти не обещан.
pub fn parts<T>(v: &Vec<T>) -> (*const T, usize, usize) {
    (v.as_ptr(), v.capacity(), v.len())
}
// ANCHOR_END: parts

// ANCHOR: growth
/// Ёмкости, через которые проходит пустой `Vec<T>` за `n` вызовов `push`.
pub fn growth<T: Copy>(x: T, n: usize) -> Vec<usize> {
    let mut v = Vec::new();
    let mut caps = alloc::vec![v.capacity()];
    for _ in 0..n {
        v.push(x);
        if v.capacity() != caps[caps.len() - 1] {
            caps.push(v.capacity());
        }
    }
    caps
}
// ANCHOR_END: growth

// ANCHOR: reserve
/// Ёмкость после каждого шага: `vec![1, 2, 3]`, `reserve(1)`, `reserve(10)`,
/// `reserve_exact(10)`, `shrink_to_fit`, `clear`.
///
/// ```
/// let caps = ch15::vec::reserve_steps();
/// // Гарантии: не меньше просьбы, после shrink_to_fit — не больше
/// // необходимого распределителю, clear ёмкость не трогает.
/// assert_eq!(caps[0], 3);
/// assert!(caps[1] >= 4 && caps[2] >= 13 && caps[3] >= 13);
/// assert_eq!(caps[5], caps[4]);
/// ```
pub fn reserve_steps() -> [usize; 6] {
    let mut v: Vec<u32> = alloc::vec![1, 2, 3];
    let mut caps = [v.capacity(); 6];
    v.reserve(1); // нужно 4, удвоение даёт 6
    caps[1] = v.capacity();
    v.reserve(10); // нужно 13 — больше удвоения
    caps[2] = v.capacity();
    v.reserve_exact(10); // места на 10 элементов сверх len уже есть
    caps[3] = v.capacity();
    v.shrink_to_fit();
    caps[4] = v.capacity();
    v.clear();
    caps[5] = v.capacity();
    caps
}
// ANCHOR_END: reserve

// ANCHOR: try_reserve
/// Буфер под `n` байт, если распределитель его даст: без паники и без
/// аварийной остановки процесса.
pub fn buffer(n: usize) -> Result<Vec<u8>, TryReserveError> {
    let mut v = Vec::new();
    v.try_reserve_exact(n)?;
    v.resize(n, 0);
    Ok(v)
}
// ANCHOR_END: try_reserve

// ANCHOR: index_instead
/// Вместо ссылки на элемент, которая не переживает `push`, хранится
/// индекс: он остаётся верным, куда бы ни переехал буфер.
pub fn bump_first_after_growth(v: &mut Vec<u32>, extra: usize) -> u32 {
    let first = 0; // не &v[0]
    for i in 0..extra {
        v.push(i as u32);
    }
    v[first] += 1;
    v[first]
}
// ANCHOR_END: index_instead

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_are_three_words() {
        assert_eq!(size_of::<Vec<u8>>(), 3 * size_of::<usize>());
        let v: Vec<u8> = Vec::with_capacity(10);
        let (_, cap, len) = parts(&v);
        assert_eq!((cap, len), (10, 0));
    }

    #[test]
    fn index_survives_growth() {
        let mut v = alloc::vec![41];
        assert_eq!(bump_first_after_growth(&mut v, 100), 42);
    }

    #[test]
    fn try_reserve_reports_both_errors() {
        // Переполнение ёмкости видно до вызова распределителя.
        assert!(buffer(usize::MAX).is_err());
        assert_eq!(buffer(16).unwrap().len(), 16);
    }
}
