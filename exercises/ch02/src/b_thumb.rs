//! Задача 2.B. Декодер подмножества 16-битных инструкций Thumb: MOV, ADD,
//! SUB, B и LDR в кодировках из `ch02::thumb::Insn` (файл
//! `code/ch02/src/thumb.rs`, у каждого варианта указана битовая маска).
//!
//! Напишите `decode` одним `match` по старшим битам слова, уточняя
//! кодировку охранными условиями и вложенными `match`. Всё, что не входит в
//! подмножество, включая `udf` и `svc`, — `None`.
//!
//! Тест сверяет `Disasm { insn, addr }.to_string()` с выводом
//! `llvm-objdump`: сначала на готовом корпусе `tests-shared/ch02-thumb.txt`,
//! затем, по `--ignored`, на дизассемблере всех прошивок из `firmware/`,
//! собранных для Cortex-M0.
//!
//! ```text
//! cargo test -p ch02-exercises task_b
//! cargo test -p ch02-exercises task_b -- --ignored
//! ```

use ch02::thumb::Insn;

/// Разбирает 16-битное слово инструкции. `None` — инструкция вне
/// подмножества (или не инструкция вовсе, как `udf`).
pub fn decode(hw: u16) -> Option<Insn> {
    todo!("задача 2.B")
}
