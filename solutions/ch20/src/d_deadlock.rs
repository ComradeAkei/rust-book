//! Задача D: взаимная блокировка и её устранение упорядоченным захватом.
//! Перевод между счетами держит два замка сразу. Если поток A берёт замки в
//! порядке (1, 2), а поток B — (2, 1), они встают навечно: каждый держит
//! один замок и ждёт второй. Лекарство — глобальный порядок: замки всегда
//! захватывают по возрастанию индекса, тогда цикла ожидания не возникает.

use std::sync::Mutex;

// ANCHOR: solution
/// Банк из нескольких счетов, каждый под своим замком.
pub struct Bank {
    accounts: Vec<Mutex<i64>>,
}

impl Bank {
    pub fn new(balances: &[i64]) -> Self {
        Bank { accounts: balances.iter().map(|&b| Mutex::new(b)).collect() }
    }

    /// Перевести `amount` со счёта `from` на `to`. Замки захватываются в
    /// порядке возрастания индекса — одинаковом для всех потоков, поэтому
    /// взаимной блокировки нет, в какую бы сторону ни шёл перевод.
    pub fn transfer(&self, from: usize, to: usize, amount: i64) {
        if from == to {
            return; // один счёт — второй замок не берём (иначе самоблокировка)
        }
        // Канонический порядок: сначала меньший индекс, потом больший.
        let (lo, hi) = (from.min(to), from.max(to));
        let mut lo_guard = self.accounts[lo].lock().unwrap();
        let mut hi_guard = self.accounts[hi].lock().unwrap();
        // Разложить, какой из захваченных охранников — источник, какой цель.
        let (src, dst) = if from == lo {
            (&mut *lo_guard, &mut *hi_guard)
        } else {
            (&mut *hi_guard, &mut *lo_guard)
        };
        *src -= amount;
        *dst += amount;
    }

    /// Сумма всех счетов — инвариант, который переводы не меняют.
    pub fn total(&self) -> i64 {
        self.accounts.iter().map(|a| *a.lock().unwrap()).sum()
    }
}
// ANCHOR_END: solution
