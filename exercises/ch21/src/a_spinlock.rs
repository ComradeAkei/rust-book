//! Задача A: спинлок на `compare_exchange_weak`. Условие — раздел 21.7.

use std::cell::UnsafeCell;
use std::sync::atomic::AtomicBool;

// ANCHOR: skeleton
#[allow(dead_code)]
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: доступ к data сериализован флагом locked.
unsafe impl<T: Send> Sync for SpinLock<T> {}

pub struct Guard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        SpinLock { locked: AtomicBool::new(false), data: UnsafeCell::new(value) }
    }

    /// Захватить замок. Крутите `compare_exchange_weak(false → true)` в цикле;
    /// успех захвата — `Ordering::Acquire` (критическая секция не уедет выше
    /// замка). Пока занято, ждите на дешёвом `load(Relaxed)` + `spin_loop`.
    pub fn lock(&self) -> Guard<'_, T> {
        todo!("раздел 21.7, задача A")
    }
}

impl<T> std::ops::Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: замок захвачен — другого доступа нет.
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
        // Освободите замок записью `false`. Порядок — `Ordering::Release`,
        // чтобы записи секции стали видны следующему захватившему (Acquire).
        todo!("раздел 21.7, задача A")
    }
}
// ANCHOR_END: skeleton
