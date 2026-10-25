//! Задача D: перевод между счетами без взаимной блокировки. Условие —
//! раздел 20.7.

use std::sync::Mutex;

// ANCHOR: skeleton
/// Банк из нескольких счетов, каждый под своим замком.
#[allow(dead_code)]
pub struct Bank {
    accounts: Vec<Mutex<i64>>,
}

impl Bank {
    pub fn new(balances: &[i64]) -> Self {
        Bank { accounts: balances.iter().map(|&b| Mutex::new(b)).collect() }
    }

    /// Перевести `amount` со счёта `from` на `to`. Чтобы встречные переводы
    /// не вставали во взаимную блокировку, захватывайте замки в ОДНОМ порядке
    /// для всех потоков — по возрастанию индекса. Случай `from == to`
    /// обработайте отдельно (второй замок того же счёта — самоблокировка).
    pub fn transfer(&self, from: usize, to: usize, amount: i64) {
        let _ = (from, to, amount);
        todo!("раздел 20.7, задача D")
    }

    /// Сумма всех счетов — инвариант, который переводы не меняют.
    pub fn total(&self) -> i64 {
        todo!("раздел 20.7, задача D")
    }
}
// ANCHOR_END: skeleton
