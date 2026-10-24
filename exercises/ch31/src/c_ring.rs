//! Задача 31.C: очередь SPSC главы 21 (раздел 21.5) со свободно бегущими
//! индексами.
//!
//! В главе 21 индексы заворачиваются по модулю `N`, и один слот всегда
//! пуст, чтобы отличить полную очередь от пустой. Здесь `head` и `tail` —
//! счётчики записанных и прочитанных элементов: их разность — длина, а
//! заняты могут быть все `N` слотов. Так часто пишут во встраиваемом коде.
//!
//! Докажите с Kani, что `push` и `pop` не переполняются, не выходят за
//! границы и сохраняют порядок при любом `N` от 1 до 8 и из любого
//! состояния — его даёт [`Ring::new_at`]. Доказательства положите в модуль
//! `proofs` под `#[cfg(kani)]` в конце файла. Найденные ошибки исправьте
//! так, чтобы очередь работала при любом `N`.

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

// ANCHOR: ring
pub struct Ring<T, const N: usize> {
    buf: [UnsafeCell<MaybeUninit<T>>; N],
    /// Сколько элементов записано за всё время.
    head: AtomicUsize,
    /// Сколько прочитано.
    tail: AtomicUsize,
}

// SAFETY: как в главе 21: писатель один и читатель один, слот переходит
// от писателя к читателю через Release-запись индекса и Acquire-чтение.
unsafe impl<T: Send, const N: usize> Sync for Ring<T, N> {}

impl<T, const N: usize> Ring<T, N> {
    pub fn new() -> Self {
        Self::new_at(0)
    }

    /// Пустая очередь в том состоянии, в каком оказалась бы `new()` после
    /// `ops` пар push и pop. Для тестов и доказательств: столько операций
    /// за время теста не сделать.
    pub fn new_at(ops: usize) -> Self {
        assert!(N >= 1, "нужен хотя бы один слот");
        Ring {
            buf: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            head: AtomicUsize::new(ops),
            tail: AtomicUsize::new(ops),
        }
    }

    /// Число элементов. Точное, пока очередь не меняют другие потоки.
    pub fn len(&self) -> usize {
        self.head.load(Ordering::Acquire) - self.tail.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&self, v: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        if head - self.tail.load(Ordering::Acquire) == N {
            return Err(v);
        }
        // SAFETY: слот head % N свободен, писатель один.
        unsafe { (*self.buf[head % N].get()).write(v) };
        self.head.store(head + 1, Ordering::Release);
        Ok(())
    }

    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: слот tail % N заполнен и опубликован, читатель один.
        let v = unsafe { (*self.buf[tail % N].get()).assume_init_read() };
        self.tail.store(tail + 1, Ordering::Release);
        Some(v)
    }
}
// ANCHOR_END: ring

impl<T, const N: usize> Default for Ring<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Drop for Ring<T, N> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}
