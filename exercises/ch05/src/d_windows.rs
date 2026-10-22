//! Задача 5.D: перекрывающиеся окна `&mut [T]` через GAT.
//!
//! Два окна сразу держать нельзя: второй `next` заимствует итератор, пока
//! первое окно ещё используется. Этот пример не должен компилироваться и
//! после решения:
//!
//! ```compile_fail,E0499
//! use ch05_exercises::d_windows::{LendingIterator, windows_mut};
//!
//! let mut v = [1, 2, 3];
//! let mut it = windows_mut(&mut v, 2);
//! let a = it.next().unwrap();
//! let b = it.next().unwrap();
//! a[1] = b[0];
//! ```

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
    todo!()
}

impl<'s, T> LendingIterator for WindowsMut<'s, T> {
    type Item<'a>
        = &'a mut [T]
    where
        Self: 'a;

    fn next(&mut self) -> Option<&mut [T]> {
        todo!()
    }
}

/// Убирает одиночные выбросы: если соседи равны, а середина отличается от
/// них больше чем на `threshold`, середина становится равной соседям.
/// Возвращает число исправленных отсчётов. Используйте `windows_mut`.
pub fn despike(samples: &mut [i32], threshold: u32) -> usize {
    todo!()
}
