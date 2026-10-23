//! Задача B: функция целиком на ассемблере через `global_asm!`. В отличие от
//! примера `ch19_triple` из раздела 19.2 здесь есть ветвление без перехода —
//! через `cmov`.

use core::arch::global_asm;

// ANCHOR: solution
// max(a, b) без ветвлений: результат в rax, по C-ABI a в rdi, b в rsi.
//   rax = a; if a < b { rax = b }   — cmovb читает CF от предыдущего cmp.
global_asm!(
    ".globl ch19_max",
    "ch19_max:",
    "    mov rax, rdi",
    "    cmp rax, rsi",
    "    cmovb rax, rsi",
    "    ret",
);

unsafe extern "C" {
    fn ch19_max(a: u64, b: u64) -> u64;
}

/// Безопасная обёртка над `ch19_max` из `global_asm!`.
pub fn max(a: u64, b: u64) -> u64 {
    // SAFETY: ch19_max — чистая функция над двумя аргументами в rdi/rsi,
    // результат в rax по C-ABI; памяти и стека не касается.
    unsafe { ch19_max(a, b) }
}
// ANCHOR_END: solution
