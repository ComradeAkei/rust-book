//! Раздел 2.6: сколько живут временные значения в `match`.
//!
//! Модуль нужен только на хосте: `Mutex` и `RefCell` с журналом берутся из
//! `std`.

use std::cell::RefCell;
use std::sync::Mutex;
use std::vec::Vec;

/// Журнал событий: в него пишут деструкторы и ветки.
pub type Log = RefCell<Vec<&'static str>>;

// ANCHOR: noisy
/// Временное значение, которое отмечает в журнале своё уничтожение.
pub struct Noisy<'a> {
    log: &'a Log,
}

impl<'a> Noisy<'a> {
    pub fn new(log: &'a Log) -> Self {
        Noisy { log }
    }

    /// Значение, по которому идёт сопоставление: копия, не ссылка.
    pub fn value(&self) -> Option<u32> {
        Some(1)
    }
}

impl Drop for Noisy<'_> {
    fn drop(&mut self) {
        self.log.borrow_mut().push("drop");
    }
}
// ANCHOR_END: noisy

// ANCHOR: scopes
/// Временное значение в выражении `match` живёт до конца всего `match`.
pub fn in_match(log: &Log) {
    match Noisy::new(log).value() {
        Some(_) => log.borrow_mut().push("arm"),
        None => log.borrow_mut().push("other arm"),
    }
}

/// Условие `if` — отдельная область: временное значение уничтожается до
/// того, как выполнится ветка.
pub fn in_if(log: &Log) {
    if Noisy::new(log).value().is_some() {
        log.borrow_mut().push("then");
    }
}

/// В `let` временное значение живёт до конца инструкции, то есть до `;`.
pub fn in_let(log: &Log) {
    let v = Noisy::new(log).value();
    log.borrow_mut().push("after let");
    if v.is_some() {
        log.borrow_mut().push("then");
    }
}
// ANCHOR_END: scopes

// ANCHOR: mutex
/// Кэш значения под мьютексом. Возвращает, занят ли мьютекс, пока
/// выполняется ветка `match`.
pub fn locked_in_arm(cache: &Mutex<Option<u32>>) -> bool {
    match *cache.lock().unwrap() {
        Some(_) => cache.try_lock().is_err(),
        None => cache.try_lock().is_err(),
    }
}

/// Значение копируется из-под замка в переменную, и guard уничтожается в
/// конце `let`. В ветках мьютекс свободен.
pub fn locked_after_let(cache: &Mutex<Option<u32>>) -> bool {
    let value = *cache.lock().unwrap();
    match value {
        Some(_) => cache.try_lock().is_err(),
        None => cache.try_lock().is_err(),
    }
}
// ANCHOR_END: mutex

#[cfg(test)]
mod tests {
    use super::*;

    // ANCHOR: tests
    #[test]
    fn match_keeps_temporary() {
        let log = Log::default();
        in_match(&log);
        assert_eq!(*log.borrow(), ["arm", "drop"]);
    }

    #[test]
    fn if_drops_before_branch() {
        let log = Log::default();
        in_if(&log);
        assert_eq!(*log.borrow(), ["drop", "then"]);
    }

    #[test]
    fn let_drops_at_semicolon() {
        let log = Log::default();
        in_let(&log);
        assert_eq!(*log.borrow(), ["drop", "after let", "then"]);
    }

    #[test]
    fn guard_lives_through_match() {
        assert!(locked_in_arm(&Mutex::new(None)));
        assert!(locked_in_arm(&Mutex::new(Some(5))));
        assert!(!locked_after_let(&Mutex::new(None)));
        assert!(!locked_after_let(&Mutex::new(Some(5))));
    }
    // ANCHOR_END: tests
}
