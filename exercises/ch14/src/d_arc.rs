//! Задача D: цена `Arc::clone`, когда клонируют из многих потоков.
//!
//! Каждая функция запускает `threads` потоков, каждый делает `iters` пар
//! `clone` + `drop` (или столько же чтений значения), и возвращает время
//! одной пары в наносекундах. Чьё время брать и кто его засекает, решите
//! сами и объясните в отчёте; `std::hint::black_box` не даст оптимизатору
//! выбросить пары целиком.

use alloc::sync::Arc;
use alloc::vec::Vec;
use std::hint::black_box;
use std::sync::Barrier;
use std::time::Instant;

/// Один `Arc` на всех: все потоки меняют один и тот же счётчик.
pub fn shared(threads: usize, iters: usize) -> f64 {
    todo!()
}

/// У каждого потока свой `Arc`.
pub fn private(threads: usize, iters: usize) -> f64 {
    todo!()
}

/// У каждого потока свой `Arc`, и счётчики соседей не делят кэш-линию.
pub fn private_padded(threads: usize, iters: usize) -> f64 {
    todo!()
}

/// Без клонов: потоки читают значение по обычной ссылке.
pub fn borrowed(threads: usize, iters: usize) -> f64 {
    todo!()
}

/// Номера кэш-линий (адрес / 64) счётчиков `n` блоков `Arc`, выделенных
/// так же, как в `private` (`padded = false`) и `private_padded`.
pub fn lines(n: usize, padded: bool) -> Vec<usize> {
    todo!()
}
