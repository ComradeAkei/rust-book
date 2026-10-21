//! Задача 2.D. Временный `MutexGuard` и взаимная блокировка.
//!
//! Пять функций делают одно и то же: если в кэше пусто, записывают туда
//! `FRESH` и возвращают его, иначе возвращают то, что лежит в кэше. Каждая
//! проверяет кэш своей конструкцией, как сказано в её описании, и каждая
//! захватывает мьютекс выражением `cache.lock().unwrap()` прямо в этой
//! конструкции, без промежуточной переменной.
//!
//! Порядок работы:
//!
//! 1. Напишите пять функций.
//! 2. Не запуская тесты, заполните `HANGS_2021` и `HANGS_2024`: зависнет
//!    ли функция на пустом кэше в каждой редакции.
//! 3. Запустите обе проверки. Тест виснущих функций ждёт полсекунды.
//!
//!    ```text
//!    cargo test -p ch02-exercises task_d
//!    cargo test -p ch02-exercises-2021
//!    ```
//!
//! 4. Напишите `fixed`, которая не виснет ни в одной редакции.
//! 5. Уберите `#[allow(clippy::correctness)]` в
//!    `exercises/ch02-2021/src/lib.rs` и запустите `cargo clippy` для обоих
//!    крейтов, затем ещё раз с `-W clippy::significant_drop_in_scrutinee`.
//!    Какие из ваших функций нашёл каждый анализатор и где он ошибся?
//!
//! Файл компилируется дважды: в `ch02-exercises` (edition 2024) и в
//! `ch02-exercises-2021` через `#[path]`. Поэтому в нём не должно быть
//! синтаксиса, которого не было в 2021, например цепочек `if let ... &&`.

use std::sync::Mutex;

/// Кэш вычисленного значения: `None` — ещё не вычислено.
pub type Cache = Mutex<Option<u32>>;

/// Значение, которое функции записывают в пустой кэш.
pub const FRESH: u32 = 42;

/// Проверка в условии `if`: `cache.lock().unwrap().is_none()`.
pub fn via_if(cache: &Cache) -> u32 {
    todo!("задача 2.D")
}

/// `if let Some(v) = *cache.lock().unwrap()`, запись в ветви `else`.
pub fn via_if_let(cache: &Cache) -> u32 {
    todo!("задача 2.D")
}

/// `match *cache.lock().unwrap()`, запись в ветви `None`.
pub fn via_match(cache: &Cache) -> u32 {
    todo!("задача 2.D")
}

/// `let Some(v) = *cache.lock().unwrap() else { … }`, запись в `else`.
pub fn via_let_else(cache: &Cache) -> u32 {
    todo!("задача 2.D")
}

/// `while let None = *cache.lock().unwrap()`, запись в теле цикла.
pub fn via_while_let(cache: &Cache) -> u32 {
    todo!("задача 2.D")
}

/// Зависнет ли функция на пустом кэше, по порядку: `via_if`,
/// `via_if_let`, `via_match`, `via_let_else`, `via_while_let`.
pub const HANGS_2021: [Option<bool>; 5] = [None; 5];
pub const HANGS_2024: [Option<bool>; 5] = [None; 5];

/// То же без взаимной блокировки в любой редакции.
pub fn fixed(cache: &Cache) -> u32 {
    todo!("задача 2.D")
}
