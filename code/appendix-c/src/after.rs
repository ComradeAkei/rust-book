//! Вектор фиксированной ёмкости после ревью по чек-листу приложения C.

use core::mem::MaybeUninit;
use core::{ptr, slice};

// ANCHOR: after
/// Вектор на стеке ёмкостью `N`, без кучи: для прошивок и обработчиков
/// прерываний.
///
/// Инвариант, на который опирается весь `unsafe` в модуле: `len <= N`, и
/// элементы `buf[..len]` инициализированы, а `buf[len..]` нет. Поля
/// приватные, поэтому нарушить инвариант может только код этого модуля.
pub struct StackVec<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> StackVec<T, N> {
    pub const fn new() -> Self {
        Self { buf: [const { MaybeUninit::uninit() }; N], len: 0 }
    }

    pub fn push(&mut self, v: T) -> Result<(), T> {
        if self.len == N {
            return Err(v);
        }
        self.buf[self.len].write(v);
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: элемент `len` был инициализирован (инвариант), а после
        // уменьшения `len` он считается неинициализированным, поэтому
        // прочитан он будет ровно один раз.
        Some(unsafe { self.buf[self.len].assume_init_read() })
    }

    pub fn as_slice(&self) -> &[T] {
        // SAFETY: `buf[..len]` инициализированы (инвариант), `MaybeUninit<T>`
        // имеет ту же раскладку, что `T`, а время жизни среза привязано к
        // `&self`, так что изменить вектор, пока срез жив, нельзя.
        unsafe { slice::from_raw_parts(self.buf.as_ptr().cast::<T>(), self.len) }
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        let last = self.len.checked_sub(1)?;
        // SAFETY: `last < len`, значит, элемент инициализирован; ссылка
        // `&mut` выводится из `&mut self` и не может пересечься с другой.
        Some(unsafe { self.buf[last].assume_init_mut() })
    }

    pub fn extend_from_slice(&mut self, src: &[T])
    where
        T: Clone,
    {
        assert!(src.len() <= N - self.len, "StackVec переполнен");
        for x in src {
            // `clone` может запаниковать. `len` растёт только после записи,
            // поэтому при панике `Drop` увидит лишь готовые элементы.
            self.buf[self.len].write(x.clone());
            self.len += 1;
        }
    }
}

impl<T, const N: usize> Drop for StackVec<T, N> {
    fn drop(&mut self) {
        let init = ptr::slice_from_raw_parts_mut(self.buf.as_mut_ptr().cast::<T>(), self.len);
        // SAFETY: элементы `buf[..len]` инициализированы (инвариант), и
        // после `drop` к ним никто не обратится. `drop_in_place` для среза
        // при панике в деструкторе одного элемента уничтожает остальные,
        // как массив и `Vec`; цикл с `assume_init_drop` бросил бы их.
        unsafe { ptr::drop_in_place(init) };
    }
}

// `Send` и `Sync` не реализованы вручную: компилятор выводит их сам из
// `[MaybeUninit<T>; N]`, то есть ровно тогда, когда их реализует `T`.
// ANCHOR_END: after

impl<T, const N: usize> Default for StackVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Вектор с `Rc` внутри больше нельзя отправить в другой поток:
///
/// ```compile_fail,E0277
/// use appendix_c::after::StackVec;
/// use std::rc::Rc;
/// let mut v = StackVec::<Rc<u8>, 1>::new();
/// v.push(Rc::new(0)).unwrap();
/// std::thread::spawn(move || drop(v));
/// ```
pub const E0277_IS_CHECKED_BY_DOCTEST: () = ();

#[cfg(test)]
mod tests {
    use super::StackVec;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn push_respects_capacity() {
        let mut v = StackVec::<u32, 2>::new();
        assert!(v.push(1).is_ok());
        assert!(v.push(2).is_ok());
        assert_eq!(v.push(3), Err(3));
        assert_eq!(v.as_slice(), [1, 2]);
        assert_eq!(v.pop(), Some(2));
        assert_eq!(v.pop(), Some(1));
        assert_eq!(v.pop(), None);
    }

    #[test]
    fn last_mut_needs_unique_access() {
        let mut v = StackVec::<u32, 4>::new();
        assert_eq!(v.last_mut(), None);
        v.push(1).unwrap();
        *v.last_mut().unwrap() = 7;
        assert_eq!(v.as_slice(), [7]);
    }

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
    fn panic_in_clone_drops_only_clones() {
        let src = [Fragile { id: 1 }, Fragile { id: 2 }];
        let result = std::panic::catch_unwind(|| {
            let mut v = StackVec::<Fragile, 4>::new();
            v.extend_from_slice(&src);
        });
        assert!(result.is_err());
        assert_eq!(*DROPPED.lock().unwrap_or_else(|e| e.into_inner()), [101]);
    }

    struct Bomb(u32);

    static BOMB_DROPS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

    impl Drop for Bomb {
        fn drop(&mut self) {
            BOMB_DROPS.lock().unwrap_or_else(|e| e.into_inner()).push(self.0);
            assert!(self.0 != 1, "drop №1 не удался");
        }
    }

    #[test]
    fn panic_in_drop_still_drops_the_rest() {
        let result = std::panic::catch_unwind(|| {
            let mut v = StackVec::<Bomb, 3>::new();
            for id in 1..=3 {
                assert!(v.push(Bomb(id)).is_ok());
            }
        });
        assert!(result.is_err());
        assert_eq!(*BOMB_DROPS.lock().unwrap_or_else(|e| e.into_inner()), [1, 2, 3]);
    }

    #[test]
    fn drops_every_element_once() {
        let counter = std::rc::Rc::new(());
        {
            let mut v = StackVec::<std::rc::Rc<()>, 3>::new();
            v.extend_from_slice(&[counter.clone(), counter.clone()]);
            v.push(counter.clone()).unwrap();
            assert_eq!(std::rc::Rc::strong_count(&counter), 4);
            drop(v.pop());
            assert_eq!(std::rc::Rc::strong_count(&counter), 3);
        }
        assert_eq!(std::rc::Rc::strong_count(&counter), 1);
    }

    #[test]
    fn send_follows_element_type() {
        fn assert_send<T: Send>() {}
        assert_send::<StackVec<u32, 4>>();
        assert_send::<StackVec<String, 4>>();
    }
}
