//! Раздел B.2: время жизни временных значений.
//!
//! Файл компилируется дважды: в крейте `appendix-b` (edition 2024) и в
//! `appendix-b-2021` через `#[path]`. Тесты каждого крейта проверяют свой
//! порядок событий, поэтому разница редакций видна на одном исходнике.

use std::cell::RefCell;
use std::sync::Mutex;

/// Журнал событий: в него пишут деструкторы и ветки `if let`.
pub type Log = RefCell<Vec<&'static str>>;

/// Временное значение, которое отмечает в журнале своё уничтожение.
pub struct Noisy<'a> {
    name: &'static str,
    log: &'a Log,
}

impl<'a> Noisy<'a> {
    pub fn new(name: &'static str, log: &'a Log) -> Self {
        Noisy { name, log }
    }

    pub fn value(&self) -> Option<u32> {
        None
    }

    pub fn name_len(&self) -> usize {
        self.name.len()
    }
}

impl Drop for Noisy<'_> {
    fn drop(&mut self) {
        self.log.borrow_mut().push(self.name);
    }
}

// ANCHOR: if_let
/// Временное значение в условии `if let`: когда оно уничтожается?
#[allow(clippy::redundant_pattern_matching)] // нужен именно `if let`
pub fn if_let_order(log: &Log) {
    if let Some(_) = Noisy::new("drop scrutinee", log).value() {
        log.borrow_mut().push("then");
    } else {
        log.borrow_mut().push("else");
    }
}
// ANCHOR_END: if_let

// ANCHOR: relock
/// Кэш значения под мьютексом. Если значения нет, ветка `else` хочет снова
/// взять тот же мьютекс, чтобы записать его. Возвращает, удалось ли это.
pub fn relock_in_else(cache: &Mutex<Option<u32>>) -> bool {
    if let Some(v) = *cache.lock().unwrap() {
        v > 0
    } else {
        // В настоящем коде здесь стоял бы `cache.lock()`, и при занятом
        // мьютексе поток ждал бы сам себя вечно. `try_lock` показывает то
        // же самое без зависания.
        cache.try_lock().is_ok()
    }
}
// ANCHOR_END: relock

// ANCHOR: tail
/// Временное значение в хвостовом выражении функции.
pub fn tail_order(log: &Log) -> usize {
    let _local = Noisy::new("drop local", log);
    Noisy::new("drop temporary", log).name_len()
}
// ANCHOR_END: tail
