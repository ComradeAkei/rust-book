//! Раздел 15.5: локальность данных. Одна и та же сумма по `Vec<u64>`, по
//! `Vec<Box<u64>>` и по `LinkedList<u64>`: в первом случае процессор читает
//! память подряд, во втором и третьем — переходит по указателям, и
//! скорость зависит от того, где распределитель положил блоки.

use std::collections::LinkedList;

// ANCHOR: sums
pub fn sum_vec(v: &[u64]) -> u64 {
    v.iter().sum()
}

pub fn sum_boxes(v: &[Box<u64>]) -> u64 {
    v.iter().map(|b| **b).sum()
}

pub fn sum_list(l: &LinkedList<u64>) -> u64 {
    l.iter().sum()
}
// ANCHOR_END: sums

// ANCHOR: shuffled
/// `n` блоков, выделенных подряд, в перемешанном порядке: указатели те же,
/// что у выделенных подряд, но обход прыгает по памяти.
pub fn shuffled_boxes(n: usize, mut seed: u64) -> Vec<Box<u64>> {
    let mut v: Vec<Box<u64>> = (0..n as u64).map(Box::new).collect();
    // Тасование Фишера — Йейтса на xorshift64.
    for i in (1..n).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        v.swap(i, (seed % (i as u64 + 1)) as usize);
    }
    v
}
// ANCHOR_END: shuffled

/// Список, узлы которого выделены вперемешку с мусором: так он выглядит в
/// долгоживущей программе, где между вставками память выделяет кто-то ещё.
pub fn scattered_list(n: usize, mut seed: u64) -> LinkedList<u64> {
    let mut l = LinkedList::new();
    let mut junk: Vec<Vec<u8>> = Vec::new();
    for i in 0..n as u64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        junk.push(vec![0; 16 + (seed % 64) as usize]);
        l.push_back(i);
    }
    drop(junk);
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_sums() {
        let n = if cfg!(miri) { 100 } else { 10_000 };
        let v: Vec<u64> = (0..n as u64).collect();
        let want = sum_vec(&v);
        assert_eq!(sum_boxes(&shuffled_boxes(n, 1)), want);
        assert_eq!(sum_list(&v.iter().copied().collect()), want);
        assert_eq!(sum_list(&scattered_list(n, 7)), want);
    }

    #[test]
    fn shuffle_is_permutation() {
        let mut got: Vec<u64> = shuffled_boxes(1000, 3).into_iter().map(|b| *b).collect();
        assert_ne!(got[..10], [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        got.sort_unstable();
        assert!(got.iter().copied().eq(0..1000));
    }
}
