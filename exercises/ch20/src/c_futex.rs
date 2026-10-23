//! Задача C: мьютекс на три состояния поверх `futex` (Linux/x86-64).
//! Условие — раздел 20.7.

use std::sync::atomic::AtomicU32;

// ANCHOR: skeleton
/// Мьютекс на три состояния: 0 — свободен, 1 — занят без ожидающих, 2 —
/// занят с ожидающими. Быстрый путь (свободен → занят) идёт одной атомарной
/// операцией без ядра; ядро (`futex`) зовут только при конкуренции.
#[allow(dead_code)]
pub struct FutexMutex {
    state: AtomicU32,
}

impl Default for FutexMutex {
    fn default() -> Self {
        Self::new()
    }
}

impl FutexMutex {
    pub const fn new() -> Self {
        FutexMutex { state: AtomicU32::new(0) }
    }

    /// Захватить замок. Быстрый путь — `compare_exchange(0 → 1)`; медленный —
    /// пометить состояние 2 и уснуть на `futex(FUTEX_WAIT_PRIVATE)`, пока
    /// значение остаётся 2. Системный вызов futex делайте инструкцией
    /// `syscall` (номер 202; аргументы в rdi/rsi/rdx/r10/r8/r9), см.
    /// [19.1](../ch19/01-asm.md).
    pub fn lock(&self) {
        todo!("раздел 20.7, задача C")
    }

    /// Освободить замок (`swap(0)`); если прежнее состояние было 2 —
    /// разбудить одного ожидающего `futex(FUTEX_WAKE_PRIVATE, 1)`.
    pub fn unlock(&self) {
        todo!("раздел 20.7, задача C")
    }
}
// ANCHOR_END: skeleton
