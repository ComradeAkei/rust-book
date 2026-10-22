//! Задача 2.A. Во что LLVM превратит пять `match` из `ch02::task_a`
//! (файл `code/ch02/src/task_a.rs`).
//!
//! Правила: сначала запишите все пять ответов и короткое объяснение к
//! каждому, не глядя в ассемблер, только потом запускайте
//! `cargo test -p ch02-exercises task_a`. Тест сам соберёт `task_a.rs` в
//! release для x86_64 и покажет код тех функций, где вы ошиблись. Свой
//! листинг можно снять так (приложение A):
//!
//! ```text
//! cargo asm -p ch02 --lib ch02a_flash_time
//! ```
//!
//! Вторая часть — пороги. Подберите их опытами с `cargo asm` и
//! `opt-level`: тест проверит их на сотне функций с разной плотностью.

use ch02::task_a::Lowering;

/// Предсказания для функций из `code/ch02/src/task_a.rs`, по порядку:
/// `Some(Lowering::JumpTable)` и так далее.
pub const PREDICTIONS: [Option<Lowering>; 5] = [
    None, // ch02a_method
    None, // ch02a_flash_time
    None, // ch02a_uart_reg
    None, // ch02a_digits
    None, // ch02a_level_letter
];

/// Наименьшее число ветвей, при котором LLVM строит таблицу переходов.
pub const MIN_CASES: u32 = 0;

/// Наименьшая плотность значений, процентов, при `opt-level` 2 и 3:
/// число ветвей, делённое на размах значений.
pub const MIN_DENSITY: u32 = 0;

/// Наименьшая плотность при `opt-level = "s"` и `"z"`.
pub const MIN_DENSITY_SIZE: u32 = 0;
