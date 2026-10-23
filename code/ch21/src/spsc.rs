//! Раздел 21.5: lock-free очередь одного писателя и одного читателя (SPSC).
//! Классический канал «из прерывания в основной цикл» на Cortex-M. Вся
//! синхронизация — на атомарных load/store индексов, БЕЗ read-modify-write:
//! поэтому очередь работает даже на Cortex-M0, где атомарного RMW нет.
//!
//! Идея модели памяти ([21.2](super::ordering)): писатель кладёт элемент в
//! слот, затем Release-записью двигает `head`; читатель Acquire-чтением видит
//! новый `head` и тем самым — уже записанный элемент. Симметрично `tail`
//! сообщает писателю об освобождённых слотах.

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

// ANCHOR: spsc
/// Кольцевая очередь на `N` слотов (вмещает `N − 1` элемент: один слот
/// всегда пуст, чтобы отличать «полна» от «пуста»). Один писатель двигает
/// `head`, один читатель — `tail`; делить между двумя потоками можно по
/// `&`, поэтому `Sync` ставим вручную.
pub struct Spsc<T, const N: usize> {
    buf: [UnsafeCell<MaybeUninit<T>>; N],
    head: AtomicUsize, // индекс записи — меняет только писатель
    tail: AtomicUsize, // индекс чтения — меняет только читатель
}

// SAFETY: доступ к каждому слоту строго разведён по времени через Acquire/
// Release на head/tail — писатель трогает слот до публикации head, читатель
// после того, как увидел эту публикацию. `Send` для T достаточно: элемент
// передаётся от одного потока другому во владение.
unsafe impl<T: Send, const N: usize> Sync for Spsc<T, N> {}

impl<T, const N: usize> Spsc<T, N> {
    pub const fn new() -> Self {
        assert!(N >= 2, "нужно хотя бы два слота");
        Spsc {
            buf: [const { UnsafeCell::new(MaybeUninit::uninit()) }; N],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Положить элемент (вызывает только писатель). `Err(v)` — очередь полна.
    pub fn push(&self, v: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed); // свой индекс — Relaxed
        let next = (head + 1) % N;
        // Acquire синхронизируется с Release-записью tail у читателя: видим,
        // что слот действительно освобождён.
        if next == self.tail.load(Ordering::Acquire) {
            return Err(v); // полна
        }
        // SAFETY: слот [head] свободен (читатель до него ещё не дошёл),
        // писатель здесь один.
        unsafe { (*self.buf[head].get()).write(v) };
        // Release публикует и запись в слот, и новый head: читатель, увидев
        // этот head через Acquire, увидит и записанный элемент.
        self.head.store(next, Ordering::Release);
        Ok(())
    }

    /// Забрать элемент (вызывает только читатель). `None` — очередь пуста.
    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed); // свой индекс — Relaxed
        // Acquire синхронизируется с Release-записью head у писателя.
        if tail == self.head.load(Ordering::Acquire) {
            return None; // пуста
        }
        // SAFETY: слот [tail] заполнен (его опубликовал писатель через
        // Release-head, который мы увидели Acquire), читатель здесь один.
        let v = unsafe { (*self.buf[tail].get()).assume_init_read() };
        // Release публикует освобождение слота писателю.
        self.tail.store((tail + 1) % N, Ordering::Release);
        Some(v)
    }
}
// ANCHOR_END: spsc

impl<T, const N: usize> Default for Spsc<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

// Непрочитанные элементы нужно уронить при уничтожении очереди.
impl<T, const N: usize> Drop for Spsc<T, N> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn single_thread_fifo() {
        let q: Spsc<u32, 4> = Spsc::new();
        assert!(q.pop().is_none());
        assert_eq!(q.push(1), Ok(()));
        assert_eq!(q.push(2), Ok(()));
        assert_eq!(q.push(3), Ok(())); // 4 слота → вмещает 3
        assert_eq!(q.push(4), Err(4)); // полна
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.push(5), Ok(()));
        assert_eq!(q.pop(), Some(3));
        assert_eq!(q.pop(), Some(5));
        assert_eq!(q.pop(), None);
    }

    #[test]
    #[cfg_attr(miri, ignore = "миллион элементов под Miri слишком долго")]
    fn producer_consumer_preserve_order() {
        const ITEMS: usize = 200_000;
        let q: Arc<Spsc<usize, 64>> = Arc::new(Spsc::new());
        let producer = {
            let q = Arc::clone(&q);
            thread::spawn(move || {
                for i in 0..ITEMS {
                    while q.push(i).is_err() {
                        std::hint::spin_loop();
                    }
                }
            })
        };
        let mut next = 0;
        while next < ITEMS {
            match q.pop() {
                Some(v) => {
                    assert_eq!(v, next); // строгий FIFO
                    next += 1;
                }
                None => std::hint::spin_loop(),
            }
        }
        producer.join().unwrap();
    }
}
