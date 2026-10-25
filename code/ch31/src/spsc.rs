//! Раздел 31.6: очередь SPSC из главы 21 (`code/ch21/src/spsc.rs`),
//! подготовленная для loom. Отличий от оригинала три: атомики и
//! `UnsafeCell` берутся из loom, когда крейт собран с `--cfg loom`;
//! доступ к слоту идёт через замыкание `with`/`with_mut` — так loom видит
//! каждое обращение; порядки доступа — параметр типа, чтобы рядом с
//! правильной очередью жила ошибочная.

use core::marker::PhantomData;
use core::mem::MaybeUninit;

// ANCHOR: shim
#[cfg(loom)]
use loom::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(not(loom))]
use core::sync::atomic::{AtomicUsize, Ordering};

/// `UnsafeCell` с API loom: указатель на содержимое — только внутри
/// замыкания. В обычной сборке обёртка ничего не стоит.
#[cfg(not(loom))]
#[derive(Debug)]
struct UnsafeCell<T>(core::cell::UnsafeCell<T>);

#[cfg(not(loom))]
impl<T> UnsafeCell<T> {
    fn new(v: T) -> Self {
        UnsafeCell(core::cell::UnsafeCell::new(v))
    }
    fn with<R>(&self, f: impl FnOnce(*const T) -> R) -> R {
        f(self.0.get())
    }
    fn with_mut<R>(&self, f: impl FnOnce(*mut T) -> R) -> R {
        f(self.0.get())
    }
}
// ANCHOR_END: shim

// ANCHOR: orderings
/// Какими операциями очередь публикует свой индекс и читает чужой.
pub trait Orderings {
    const PUBLISH: Ordering;
    const OBSERVE: Ordering;
}

/// Как в главе 21: Release-запись индекса, Acquire-чтение.
pub struct AcqRel;

impl Orderings for AcqRel {
    const PUBLISH: Ordering = Ordering::Release;
    const OBSERVE: Ordering = Ordering::Acquire;
}

/// Ошибка: «на x86 и так работает» — всё Relaxed.
pub struct AllRelaxed;

impl Orderings for AllRelaxed {
    const PUBLISH: Ordering = Ordering::Relaxed;
    const OBSERVE: Ordering = Ordering::Relaxed;
}
// ANCHOR_END: orderings

// ANCHOR: spsc
pub struct Spsc<T, const N: usize, O: Orderings = AcqRel> {
    buf: [UnsafeCell<MaybeUninit<T>>; N],
    head: AtomicUsize,
    tail: AtomicUsize,
    orderings: PhantomData<O>,
}

// SAFETY: как в главе 21 — при порядках AcqRel доступ к слоту разведён по
// времени публикацией индексов. У AllRelaxed этого нет, и именно это
// находят loom, Miri и TSan.
unsafe impl<T: Send, const N: usize, O: Orderings> Sync for Spsc<T, N, O> {}

impl<T, const N: usize, O: Orderings> Spsc<T, N, O> {
    pub fn new() -> Self {
        assert!(N >= 2, "нужно хотя бы два слота");
        Spsc {
            buf: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            orderings: PhantomData,
        }
    }

    /// # Safety
    ///
    /// Как в главе 21: писатель один, вызовы `push` не идут одновременно.
    pub unsafe fn push(&self, v: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        let next = (head + 1) % N;
        if next == self.tail.load(O::OBSERVE) {
            return Err(v);
        }
        // SAFETY: слот [head] свободен, писатель один.
        self.buf[head].with_mut(|slot| unsafe { (*slot).write(v) });
        self.head.store(next, O::PUBLISH);
        Ok(())
    }

    /// # Safety
    ///
    /// Как в главе 21: читатель один, вызовы `pop` не идут одновременно.
    pub unsafe fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(O::OBSERVE) {
            return None;
        }
        // SAFETY: слот [tail] заполнен и опубликован, читатель один.
        let v = self.buf[tail].with(|slot| unsafe { (*slot).assume_init_read() });
        self.tail.store((tail + 1) % N, O::PUBLISH);
        Some(v)
    }
}
// ANCHOR_END: spsc

impl<T, const N: usize, O: Orderings> Default for Spsc<T, N, O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize, O: Orderings> Drop for Spsc<T, N, O> {
    fn drop(&mut self) {
        // SAFETY: `&mut self` — читатель здесь единственный.
        while unsafe { self.pop() }.is_some() {}
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;

    #[test]
    fn fifo_like_chapter_21() {
        let q: Spsc<u32, 4> = Spsc::new();
        // SAFETY: поток один — он и писатель, и читатель.
        unsafe {
            assert_eq!(q.pop(), None);
            assert_eq!(
                (q.push(1), q.push(2), q.push(3), q.push(4)),
                (Ok(()), Ok(()), Ok(()), Err(4))
            );
            assert_eq!((q.pop(), q.pop()), (Some(1), Some(2)));
            assert_eq!(q.push(5), Ok(()));
            assert_eq!((q.pop(), q.pop(), q.pop()), (Some(3), Some(5), None));
        }
    }

    /// Непрочитанные элементы роняются вместе с очередью.
    #[test]
    fn drop_releases_items() {
        let item = std::rc::Rc::new(());
        let q: Spsc<_, 3, AllRelaxed> = Spsc::new();
        // SAFETY: поток один.
        unsafe {
            q.push(item.clone()).unwrap();
            q.push(item.clone()).unwrap();
        }
        assert_eq!(std::rc::Rc::strong_count(&item), 3);
        drop(q);
        assert_eq!(std::rc::Rc::strong_count(&item), 1);
    }
}
