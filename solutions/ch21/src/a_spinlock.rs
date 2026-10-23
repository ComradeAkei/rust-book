//! Задача A: спинлок на `compare_exchange_weak`. Простейший замок: один
//! атомарный флаг, который захватывают сменой `false → true`. Весь смысл — в
//! порядке: захват должен быть Acquire (чтобы критическая секция не
//! «уехала» выше замка), освобождение — Release (чтобы её записи стали видны
//! следующему захватившему). Это прямое применение happens-before из
//! раздела 21.2.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

// ANCHOR: solution
/// Спинлок, владеющий данными `T` (как `Mutex<T>`, но без усыпления потока:
/// ожидающий крутится в цикле).
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: доступ к data сериализован флагом locked; делить &SpinLock между
// потоками безопасно, когда T можно переслать между ними.
unsafe impl<T: Send> Sync for SpinLock<T> {}

/// Охранник: пока жив, замок захвачен; по `Drop` — освобождается.
pub struct Guard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        SpinLock { locked: AtomicBool::new(false), data: UnsafeCell::new(value) }
    }

    /// Захватить замок, крутясь до успеха. `compare_exchange_weak` в цикле:
    /// на ARM `strex` вправе ложно не сработать, поэтому именно `_weak` и
    /// именно в цикле. Успех — Acquire: ничто из секции не уедет выше.
    pub fn lock(&self) -> Guard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Пока занято — ждём на дешёвом чтении, без дорогого CAS, и
            // подсказываем процессору, что это спин (pause/yield).
            while self.locked.load(Ordering::Relaxed) {
                std::hint::spin_loop();
            }
        }
        Guard { lock: self }
    }
}

impl<T> std::ops::Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: замок захвачен, другого доступа к data нет.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> std::ops::DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: замок захвачен эксклюзивно этим охранником.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        // Release: все записи секции видны следующему захватившему (Acquire).
        self.lock.locked.store(false, Ordering::Release);
    }
}
// ANCHOR_END: solution
