//! Задача 6.A. Предскажите `size_of` и `align_of` для 15 типов из списка
//! `ch06::task_a::QUESTIONS` (файл `code/ch06/src/task_a.rs`) для цели
//! x86_64-unknown-linux-gnu.
//!
//! Правила: сначала запишите все 15 ответов и короткое объяснение к каждому,
//! только потом запускайте `cargo test -p ch06-exercises task_a`.
//! Тест покажет, где вы ошиблись. Для каждой ошибки найдите правило из
//! главы, которое вы не учли.

/// Пары `(size_of, align_of)` в порядке списка.
pub const ANSWERS: [(usize, usize); 15] = [
    (0, 0), // 1  Option<Option<bool>>
    (0, 0), // 2  Result<(), Box<u8>>
    (0, 0), // 3  Option<char>
    (0, 0), // 4  Option<u32>
    (0, 0), // 5  Option<NonZeroU32>
    (0, 0), // 6  (u8, u32, u8)
    (0, 0), // 7  Mixed
    (0, 0), // 8  Packed
    (0, 0), // 9  Status
    (0, 0), // 10 Shape
    (0, 0), // 11 Option<*const u8>
    (0, 0), // 12 Option<Box<dyn Fn()>>
    (0, 0), // 13 Option<Cell<bool>>
    (0, 0), // 14 [u64; 0]
    (0, 0), // 15 Option<[bool; 3]>
];
