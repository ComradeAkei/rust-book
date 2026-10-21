//! Раздел 6.5: типы нулевого размера.

use core::marker::PhantomData;

// ANCHOR: markers
/// Состояния вывода GPIO как типы без данных. Значение `Pin<Output>` занимает
/// столько же, сколько номер вывода, а состояние существует только для
/// проверки типов и исчезает при компиляции.
pub struct Input;
pub struct Output;

pub struct Pin<Mode> {
    number: u8,
    _mode: PhantomData<Mode>,
}

impl Pin<Input> {
    pub const fn new(number: u8) -> Self {
        Pin { number, _mode: PhantomData }
    }

    /// Перевод в режим выхода поглощает старое значение: вывод нельзя
    /// использовать как вход после того, как он стал выходом.
    pub fn into_output(self) -> Pin<Output> {
        Pin { number: self.number, _mode: PhantomData }
    }
}

impl Pin<Output> {
    /// Метод существует только у выходов. `Pin<Input>::set_high` не скомпилируется.
    pub fn set_high(&mut self) -> u32 {
        1 << self.number
    }
}
// ANCHOR_END: markers

// ANCHOR: zst_array
/// Массив нулевой длины не занимает места, но сохраняет выравнивание `T`.
/// Так выравнивают начало структуры, не добавляя полей.
#[repr(C)]
pub struct AlignedTo8 {
    _align: [u64; 0],
    pub bytes: [u8; 3],
}
// ANCHOR_END: zst_array

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use core::mem::{align_of, size_of};
    use std::collections::{HashMap, HashSet};
    use std::vec::Vec;

    #[test]
    fn markers_are_free() {
        assert_eq!(size_of::<Input>(), 0);
        assert_eq!(size_of::<PhantomData<u64>>(), 0);
        assert_eq!(size_of::<Pin<Output>>(), 1);
        let mut p = Pin::new(5).into_output();
        assert_eq!(p.set_high(), 0b10_0000);
    }

    #[test]
    fn zero_length_array_keeps_align() {
        assert_eq!(size_of::<[u64; 0]>(), 0);
        assert_eq!(align_of::<[u64; 0]>(), 8);
        assert_eq!(align_of::<AlignedTo8>(), 8);
        assert_eq!(size_of::<AlignedTo8>(), 8);
    }

    // ANCHOR: vec_unit
    #[test]
    fn vec_of_unit_never_allocates() {
        // Под Miri миллион итераций идёт минуты, тысячи хватает для проверки.
        let n = if cfg!(miri) { 1_000 } else { 1_000_000 };
        let mut v: Vec<()> = Vec::new();
        for _ in 0..n {
            v.push(());
        }
        assert_eq!(v.len(), n);
        // Ёмкость ZST-вектора равна usize::MAX: памяти он не просит никогда.
        assert_eq!(v.capacity(), usize::MAX);
    }

    #[test]
    fn set_is_map_with_unit_values() {
        // HashSet<K> в std устроен как HashMap<K, ()>: место под () не нужно.
        assert_eq!(size_of::<(u32, ())>(), size_of::<u32>());
        let s: HashSet<u32> = [1, 2, 3].into_iter().collect();
        let m: HashMap<u32, ()> = [1, 2, 3].into_iter().map(|k| (k, ())).collect();
        assert_eq!(s.len(), m.len());
    }
    // ANCHOR_END: vec_unit

    #[test]
    fn distinct_zst_addresses_are_not_promised() {
        // Два разных ZST могут иметь одинаковый адрес: сравнивать адреса
        // ZST-значений, чтобы различать объекты, нельзя.
        let a = ();
        let b = ();
        let _same_or_not = core::ptr::eq(&a, &b);
    }
}
