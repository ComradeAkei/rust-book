//! Задача 2.D: временный `MutexGuard` и взаимная блокировка.
//!
//! Файл компилируется дважды: в `ch02-solutions` (edition 2024) и в
//! `ch02-solutions-2021` через `#[path]`. Поэтому в нём нет синтаксиса,
//! которого не было в 2021.

use std::sync::Mutex;

/// Кэш вычисленного значения: `None` — ещё не вычислено.
pub type Cache = Mutex<Option<u32>>;

/// Значение, которое функции записывают в пустой кэш.
pub const FRESH: u32 = 42;

// ANCHOR: variants
/// Условие `if` — отдельная область временных значений.
pub fn via_if(cache: &Cache) -> u32 {
    if cache.lock().unwrap().is_none() {
        *cache.lock().unwrap() = Some(FRESH);
        FRESH
    } else {
        cache.lock().unwrap().unwrap()
    }
}

#[allow(clippy::redundant_pattern_matching)] // нужен именно `if let`
pub fn via_if_let(cache: &Cache) -> u32 {
    if let Some(v) = *cache.lock().unwrap() {
        v
    } else {
        *cache.lock().unwrap() = Some(FRESH);
        FRESH
    }
}

pub fn via_match(cache: &Cache) -> u32 {
    match *cache.lock().unwrap() {
        Some(v) => v,
        None => {
            *cache.lock().unwrap() = Some(FRESH);
            FRESH
        }
    }
}

pub fn via_let_else(cache: &Cache) -> u32 {
    let Some(v) = *cache.lock().unwrap() else {
        *cache.lock().unwrap() = Some(FRESH);
        return FRESH;
    };
    v
}

/// Пока значения нет, записать его: тело выполнится не больше одного раза.
#[allow(clippy::redundant_pattern_matching)] // нужен именно `while let`
pub fn via_while_let(cache: &Cache) -> u32 {
    while let None = *cache.lock().unwrap() {
        *cache.lock().unwrap() = Some(FRESH);
    }
    cache.lock().unwrap().unwrap()
}
// ANCHOR_END: variants

// ANCHOR: predictions
/// Зависнет ли функция на пустом кэше, по порядку: `via_if`,
/// `via_if_let`, `via_match`, `via_let_else`, `via_while_let`.
pub const HANGS_2021: [Option<bool>; 5] =
    [Some(false), Some(true), Some(true), Some(false), Some(true)];
pub const HANGS_2024: [Option<bool>; 5] =
    [Some(false), Some(false), Some(true), Some(false), Some(true)];
// ANCHOR_END: predictions

// ANCHOR: fixed
/// Один захват мьютекса на всё: guard живёт в переменной, проверка и
/// запись идут под одним замком, и между ними другой поток не вклинится.
pub fn fixed(cache: &Cache) -> u32 {
    let mut guard = cache.lock().unwrap();
    *guard.get_or_insert(FRESH)
}
// ANCHOR_END: fixed
