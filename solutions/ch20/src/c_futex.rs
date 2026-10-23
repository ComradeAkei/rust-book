//! Задача C: мьютекс на три состояния поверх системного вызова `futex`
//! (Linux, x86-64). Три состояния снимают лишние системные вызовы: по
//! незанятому замку lock/unlock идут целиком в пространстве пользователя,
//! ядро зовут только при реальной конкуренции. Сам `futex` вызываем прямой
//! инструкцией `syscall` ([19.1](../ch19/01-asm.md)) — без крейта libc.

use std::sync::atomic::{AtomicU32, Ordering};

// ANCHOR: solution
const UNLOCKED: u32 = 0; // свободен
const LOCKED: u32 = 1; // занят, ожидающих нет
const CONTENDED: u32 = 2; // занят, есть ожидающие (ядро обязано разбудить)

const SYS_FUTEX: u64 = 202;
const FUTEX_WAIT_PRIVATE: u32 = 128; // FUTEX_WAIT | FUTEX_PRIVATE_FLAG
const FUTEX_WAKE_PRIVATE: u32 = 129; // FUTEX_WAKE | FUTEX_PRIVATE_FLAG

/// Обёртка над системным вызовом `futex(uaddr, op, val, NULL, NULL, 0)`.
///
/// # Safety
///
/// `uaddr` указывает на живой `AtomicU32`. Остальные аргументы фиксированы
/// (таймаут NULL), поэтому вызов безопасен для WAIT/WAKE над этим адресом.
unsafe fn futex(uaddr: *const AtomicU32, op: u32, val: u32) {
    // SAFETY: соглашение системного вызова Linux x86-64: номер в rax,
    // аргументы в rdi/rsi/rdx/r10/r8/r9; rcx и r11 портит сама инструкция.
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") SYS_FUTEX => _,
            in("rdi") uaddr,
            in("rsi") op,
            in("rdx") val,
            in("r10") 0u64, // timeout = NULL
            in("r8") 0u64,
            in("r9") 0u64,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
}

/// Мьютекс на три состояния. Поле `state` — та память, на которой засыпает и
/// просыпается `futex`.
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
        FutexMutex { state: AtomicU32::new(UNLOCKED) }
    }

    /// Захватить замок. Быстрый путь — одна атомарная операция без ядра.
    pub fn lock(&self) {
        // Быстрый путь: было свободно — заняли, ядро не трогаем.
        if self
            .state
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return;
        }
        // Медленный путь: помечаем «занят с ожидающими» и спим на futex.
        // swap возвращает прежнее состояние: если UNLOCKED — замок наш.
        while self.state.swap(CONTENDED, Ordering::Acquire) != UNLOCKED {
            // SAFETY: state жив, пока жив self; WAIT уснёт, только пока
            // значение всё ещё CONTENDED (ядро проверит атомарно).
            unsafe { futex(&self.state, FUTEX_WAIT_PRIVATE, CONTENDED) };
        }
    }

    /// Освободить замок и, если были ожидающие, разбудить одного.
    pub fn unlock(&self) {
        // Если состояние было CONTENDED — кто-то спит, его надо разбудить.
        if self.state.swap(UNLOCKED, Ordering::Release) == CONTENDED {
            // SAFETY: state жив; будим не более одного ожидающего.
            unsafe { futex(&self.state, FUTEX_WAKE_PRIVATE, 1) };
        }
    }
}
// ANCHOR_END: solution
