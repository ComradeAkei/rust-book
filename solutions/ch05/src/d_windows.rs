//! Задача 5.D: перекрывающиеся окна `&mut [T]` через GAT.
//!
//! Два окна сразу держать нельзя: второй `next` заимствует итератор, пока
//! первое окно ещё используется.
//!
//! ```compile_fail,E0499
//! use ch05_solutions::d_windows::{LendingIterator, windows_mut};
//!
//! let mut v = [1, 2, 3];
//! let mut it = windows_mut(&mut v, 2);
//! let a = it.next().unwrap();
//! let b = it.next().unwrap();
//! a[1] = b[0];
//! ```

// ANCHOR: lending
/// Одалживающий итератор: элемент заимствует итератор до следующего `next`.
pub trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}

/// Окна длины `size` со сдвигом на один элемент.
pub struct WindowsMut<'s, T> {
    slice: &'s mut [T],
    size: usize,
    pos: usize,
}

/// Как `slice.windows(size)`, но окна изменяемые. Паникует при `size == 0`.
pub fn windows_mut<T>(slice: &mut [T], size: usize) -> WindowsMut<'_, T> {
    assert!(size > 0, "размер окна должен быть больше нуля");
    WindowsMut { slice, size, pos: 0 }
}

impl<'s, T> LendingIterator for WindowsMut<'s, T> {
    type Item<'a>
        = &'a mut [T]
    where
        Self: 'a;

    fn next(&mut self) -> Option<&mut [T]> {
        let end = self.pos.checked_add(self.size)?;
        let w = self.slice.get_mut(self.pos..end)?;
        self.pos += 1;
        Some(w)
    }
}
// ANCHOR_END: lending

// ANCHOR: despike
/// Убирает одиночные выбросы: если соседи равны, а середина отличается от
/// них больше чем на `threshold`, середина становится равной соседям.
/// Окна перекрываются, и исправленный отсчёт виден следующему окну.
pub fn despike(samples: &mut [i32], threshold: u32) -> usize {
    let mut fixed = 0;
    let mut it = windows_mut(samples, 3);
    while let Some(w) = it.next() {
        if w[0] == w[2] && w[1].abs_diff(w[0]) > threshold {
            w[1] = w[0];
            fixed += 1;
        }
    }
    fixed
}
// ANCHOR_END: despike
