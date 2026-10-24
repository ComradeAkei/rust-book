//! Задача 31.C: очередь SPSC главы 21 со свободно бегущими индексами —
//! исправленная.
//!
//! Kani нашёл в заготовке две ошибки. Первая — переполнение: `head + 1`
//! у `usize::MAX`. После замены на `wrapping_add` вторая: при `N`, не
//! делящем 2⁶⁴ (3, 5, 6, 7), слот `head % N` после заворота счётчика
//! прыгает, и `push` пишет поверх непрочитанного элемента. Тестом этого
//! не увидеть: нужно 2⁶⁴ операций. Исправление — индексы по модулю `2N`:
//! они различают полную очередь и пустую, как свободно бегущие, а
//! заворачиваются в ноль ровно на границе слотов при любом `N`.

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

// ANCHOR: ring
pub struct Ring<T, const N: usize> {
    buf: [UnsafeCell<MaybeUninit<T>>; N],
    /// Сколько элементов записано, по модулю `2N`.
    head: AtomicUsize,
    /// Сколько прочитано, по модулю `2N`.
    tail: AtomicUsize,
}

// SAFETY: как в главе 21: писатель один и читатель один, слот переходит
// от писателя к читателю через Release-запись индекса и Acquire-чтение.
unsafe impl<T: Send, const N: usize> Sync for Ring<T, N> {}

impl<T, const N: usize> Ring<T, N> {
    /// Период индексов. `2N` не переполняется: это проверяет компилятор.
    const WRAP: usize = {
        assert!(N >= 1 && N <= usize::MAX / 2, "N от 1 до usize::MAX / 2");
        2 * N
    };

    const fn next(i: usize) -> usize {
        if i + 1 == Self::WRAP { 0 } else { i + 1 }
    }

    const fn slot(i: usize) -> usize {
        if i < N { i } else { i - N }
    }

    /// Сколько элементов между индексами: оба меньше `2N`, разность — по
    /// модулю `2N`, и ни одна операция не переполняется.
    const fn distance(head: usize, tail: usize) -> usize {
        if head >= tail { head - tail } else { Self::WRAP - (tail - head) }
    }

    pub fn new() -> Self {
        Self::new_at(0)
    }

    /// Пустая очередь в том состоянии, в каком оказалась бы `new()` после
    /// `ops` пар push и pop. Для тестов и доказательств.
    pub fn new_at(ops: usize) -> Self {
        let i = ops % Self::WRAP;
        Ring {
            buf: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            head: AtomicUsize::new(i),
            tail: AtomicUsize::new(i),
        }
    }

    /// Число элементов. Точное, пока очередь не меняют другие потоки.
    pub fn len(&self) -> usize {
        Self::distance(self.head.load(Ordering::Acquire), self.tail.load(Ordering::Acquire))
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&self, v: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        if Self::distance(head, self.tail.load(Ordering::Acquire)) == N {
            return Err(v);
        }
        // SAFETY: слот head свободен, писатель один.
        unsafe { (*self.buf[Self::slot(head)].get()).write(v) };
        self.head.store(Self::next(head), Ordering::Release);
        Ok(())
    }

    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: слот tail заполнен и опубликован, читатель один.
        let v = unsafe { (*self.buf[Self::slot(tail)].get()).assume_init_read() };
        self.tail.store(Self::next(tail), Ordering::Release);
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

/// Заготовка задачи — тот же файл, что в `exercises/ch31`: на ней Kani
/// находит переполнение.
#[cfg(kani)]
#[path = "../../../exercises/ch31/src/c_ring.rs"]
mod original;

/// Заготовка после первой правки: счётчики заворачиваются, но слот
/// по-прежнему `head % N`.
#[cfg(kani)]
mod wrapping {
    use core::cell::UnsafeCell;
    use core::mem::MaybeUninit;
    use core::sync::atomic::{AtomicUsize, Ordering};

    // ANCHOR: wrapping
    pub struct Ring<T, const N: usize> {
        buf: [UnsafeCell<MaybeUninit<T>>; N],
        head: AtomicUsize,
        tail: AtomicUsize,
    }

    impl<T, const N: usize> Ring<T, N> {
        pub fn new_at(ops: usize) -> Self {
            Ring {
                buf: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
                head: AtomicUsize::new(ops),
                tail: AtomicUsize::new(ops),
            }
        }

        pub fn len(&self) -> usize {
            self.head.load(Ordering::Acquire).wrapping_sub(self.tail.load(Ordering::Acquire))
        }

        pub fn push(&self, v: T) -> Result<(), T> {
            let head = self.head.load(Ordering::Relaxed);
            if head.wrapping_sub(self.tail.load(Ordering::Acquire)) == N {
                return Err(v);
            }
            // SAFETY: как в заготовке.
            unsafe { (*self.buf[head % N].get()).write(v) };
            self.head.store(head.wrapping_add(1), Ordering::Release);
            Ok(())
        }

        pub fn pop(&self) -> Option<T> {
            let tail = self.tail.load(Ordering::Relaxed);
            if tail == self.head.load(Ordering::Acquire) {
                return None;
            }
            // SAFETY: как в заготовке.
            let v = unsafe { (*self.buf[tail % N].get()).assume_init_read() };
            self.tail.store(tail.wrapping_add(1), Ordering::Release);
            Some(v)
        }
    }
    // ANCHOR_END: wrapping
}

#[cfg(kani)]
mod proofs {
    // ANCHOR: proofs
    /// Из любого состояния — `new_at` с любым числом прошлых операций —
    /// положить `k ≤ N` элементов, убедиться, что очередь полна ровно при
    /// `k = N`, и забрать их в том же порядке. Kani ищет и переполнения, и
    /// выход за границы массива, и нарушение `assert!`.
    macro_rules! fifo {
        ($($name:ident: $n:literal),*) => {$(
            #[kani::proof]
            #[kani::unwind(10)]
            fn $name() {
                let q = Ring::<u8, $n>::new_at(kani::any());
                let k = kani::any_where(|&k: &usize| k <= $n);
                for i in 0..k {
                    assert!(q.push(i as u8).is_ok());
                }
                assert!(q.len() == k);
                assert!(q.push(0xFF).is_err() == (k == $n));
                for i in 0..k {
                    assert!(q.pop() == Some(i as u8));
                }
            }
        )*};
    }

    macro_rules! sizes {
        () => {
            fifo!(n1: 1, n2: 2, n3: 3, n4: 4, n5: 5, n6: 6, n7: 7, n8: 8);
        };
    }
    // ANCHOR_END: proofs

    mod fixed {
        use super::super::Ring;
        sizes!();
    }

    /// Переполнение `head + 1` в заготовке от `N` не зависит, и одного
    /// размера достаточно. На степенях двойки `% N` — маска, и CBMC
    /// разбирает эту версию за секунды; при `N` = 3 остаток от деления
    /// стоит около двух минут, при 5, 6, 7 — дольше четырёх минут на
    /// размер.
    mod original {
        use super::super::original::Ring;
        fifo!(n3: 3);
    }

    mod wrapping {
        use super::super::wrapping::Ring;
        sizes!();
    }
}
