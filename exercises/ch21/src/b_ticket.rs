//! Задача B: справедливый ticket lock. Условие — раздел 21.7.

use std::cell::UnsafeCell;
use std::sync::atomic::AtomicUsize;

// ANCHOR: skeleton
#[allow(dead_code)]
pub struct TicketLock<T> {
    next: AtomicUsize,    // выдать следующий номер
    serving: AtomicUsize, // кого обслуживают сейчас
    data: UnsafeCell<T>,
}

// SAFETY: доступ к data сериализован номерками.
unsafe impl<T: Send> Sync for TicketLock<T> {}

pub struct Guard<'a, T> {
    lock: &'a TicketLock<T>,
}

impl<T> TicketLock<T> {
    pub const fn new(value: T) -> Self {
        TicketLock {
            next: AtomicUsize::new(0),
            serving: AtomicUsize::new(0),
            data: UnsafeCell::new(value),
        }
    }

    /// Взять номерок `next.fetch_add(1, Relaxed)` и ждать, пока
    /// `serving.load(Acquire)` сравняется с ним (`spin_loop` в цикле).
    pub fn lock(&self) -> Guard<'_, T> {
        todo!("раздел 21.7, задача B")
    }
}

impl<T> std::ops::Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: наш номер обслуживается — доступ эксклюзивный.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> std::ops::DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: наш номер обслуживается — доступ эксклюзивный.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        // Позовите следующий номер: `serving` на единицу больше, порядок
        // `Ordering::Release`.
        todo!("раздел 21.7, задача B")
    }
}
// ANCHOR_END: skeleton
