//! Вектор фиксированной ёмкости до ревью. Не используйте этот код: в нём
//! четыре ошибки, разобранные в приложении C.

use core::mem::MaybeUninit;
use core::slice;

// ANCHOR: before
/// Вектор на стеке ёмкостью `N`, без кучи: для прошивок и обработчиков
/// прерываний.
pub struct StackVec<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> StackVec<T, N> {
    pub const fn new() -> Self {
        Self { buf: [const { MaybeUninit::uninit() }; N], len: 0 }
    }

    pub fn push(&mut self, v: T) -> Result<(), T> {
        if self.len > N {
            return Err(v);
        }
        unsafe { self.buf.as_mut_ptr().add(self.len).cast::<T>().write(v) };
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(unsafe { self.buf[self.len].assume_init_read() })
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.buf.as_ptr().cast::<T>(), self.len) }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn last_mut(&self) -> Option<&mut T> {
        if self.len == 0 {
            return None;
        }
        unsafe { Some(&mut *(self.buf.as_ptr().add(self.len - 1) as *mut T)) }
    }

    pub fn extend_from_slice(&mut self, src: &[T])
    where
        T: Clone,
    {
        assert!(self.len + src.len() <= N);
        let start = self.len;
        self.len += src.len();
        for (i, x) in src.iter().enumerate() {
            unsafe { self.buf.as_mut_ptr().add(start + i).cast::<T>().write(x.clone()) };
        }
    }
}

impl<T, const N: usize> Drop for StackVec<T, N> {
    fn drop(&mut self) {
        for x in &mut self.buf[..self.len] {
            unsafe { x.assume_init_drop() };
        }
    }
}

unsafe impl<T, const N: usize> Send for StackVec<T, N> {}
// ANCHOR_END: before

impl<T, const N: usize> Default for StackVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::StackVec;
    use std::rc::Rc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    // ANCHOR: before_tests
    #[test]
    #[ignore = "ошибка в коде до ревью"]
    fn before_push_past_capacity() {
        let mut v = StackVec::<u32, 2>::new();
        assert!(v.push(1).is_ok());
        assert!(v.push(2).is_ok());
        assert_eq!(v.push(3), Err(3));
    }

    #[test]
    #[ignore = "UB: запускать под Miri"]
    fn before_last_mut_through_shared() {
        let mut v = StackVec::<u32, 4>::new();
        v.push(1).unwrap();
        let r = &v;
        *r.last_mut().unwrap() = 7;
        assert_eq!(v.as_slice(), [7]);
    }

    /// Значение, второй `clone` которого паникует. `Drop` записывает `id`
    /// в журнал, так что видно, какие значения были уничтожены.
    struct Fragile {
        id: u32,
    }

    static CLONES: AtomicU32 = AtomicU32::new(0);
    static DROPPED: Mutex<Vec<u32>> = Mutex::new(Vec::new());

    impl Clone for Fragile {
        fn clone(&self) -> Self {
            let n = CLONES.fetch_add(1, Ordering::Relaxed) + 1;
            assert!(n < 2, "clone №2 не удался");
            Fragile { id: self.id + 100 }
        }
    }

    impl Drop for Fragile {
        fn drop(&mut self) {
            DROPPED.lock().unwrap_or_else(|e| e.into_inner()).push(self.id);
        }
    }

    #[test]
    #[ignore = "UB: запускать под Miri"]
    fn before_panic_in_clone() {
        let src = [Fragile { id: 1 }, Fragile { id: 2 }];
        let result = std::panic::catch_unwind(|| {
            let mut v = StackVec::<Fragile, 4>::new();
            v.extend_from_slice(&src);
        });
        assert!(result.is_err());
        // Успел появиться один клон, id 101, и уничтожен должен быть только он.
        assert_eq!(*DROPPED.lock().unwrap_or_else(|e| e.into_inner()), [101]);
    }

    #[test]
    #[ignore = "UB: запускать под Miri"]
    fn before_send_with_rc() {
        let shared = Rc::new(0u8);
        let mut v = StackVec::<Rc<u8>, 1>::new();
        v.push(Rc::clone(&shared)).unwrap();
        let t = std::thread::spawn(move || {
            for _ in 0..100 {
                drop(Rc::clone(&v.as_slice()[0]));
            }
        });
        for _ in 0..100 {
            drop(Rc::clone(&shared));
        }
        t.join().unwrap();
    }
    // ANCHOR_END: before_tests
}
