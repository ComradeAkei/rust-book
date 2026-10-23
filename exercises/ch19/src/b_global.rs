//! Задача B: функция целиком на ассемблере через `global_asm!`. Условие —
//! раздел 19.7.

// ANCHOR: skeleton
// Напишите в `global_asm!` функцию `ch19_max(a, b) -> max(a, b)` без
// ветвлений (через `cmp` + `cmov`), объявите её в `unsafe extern "C"` и
// вызовите из безопасной обёртки `max`. По C-ABI a в rdi, b в rsi, результат
// в rax.

/// Безопасная обёртка над функцией `max` из `global_asm!`.
pub fn max(a: u64, b: u64) -> u64 {
    let _ = (a, b);
    todo!("раздел 19.7, задача B")
}
// ANCHOR_END: skeleton
