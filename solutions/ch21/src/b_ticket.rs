//! Задача B: справедливый ticket lock. Спинлок на `compare_exchange`
//! ([задача A](../ch21/07-tasks.md)) несправедлив: кто первым успел CAS, тот
//! и внутри, очереди нет. Ticket lock выдаёт номерки, как в поликлинике:
//! `fetch_add` берёт следующий номер, а зовут строго по возрастанию. Так
//! потоки входят в порядке прихода — без голодания.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

// ANCHOR: solution
/// Справедливый замок на двух счётчиках: `next` — номер следующего
/// претендента, `serving` — номер, которого сейчас обслуживают.
pub struct TicketLock<T> {
    next: AtomicUsize,    // выдать следующий номер
    serving: AtomicUsize, // кого обслуживают сейчас
    data: UnsafeCell<T>,
}

// SAFETY: доступ к data сериализован номерками; делить &TicketLock между
// потоками безопасно при `T: Send`.
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

    /// Взять номерок и ждать, пока вызовут именно его. `fetch_add` берёт
    /// номер неделимо (Relaxed — важна только атомарность выдачи). Вход —
    /// Acquire-чтение `serving`: синхронизируется с Release-выдачей
    /// предыдущего держателя, поэтому его записи видны нам.
    pub fn lock(&self) -> Guard<'_, T> {
        let ticket = self.next.fetch_add(1, Ordering::Relaxed);
        while self.serving.load(Ordering::Acquire) != ticket {
            std::hint::spin_loop();
        }
        Guard { lock: self }
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
        // Позвать следующий номер. Release: записи секции видны тому, кто
        // увидит новый serving Acquire-чтением.
        let next = self.lock.serving.load(Ordering::Relaxed) + 1;
        self.lock.serving.store(next, Ordering::Release);
    }
}
// ANCHOR_END: solution
