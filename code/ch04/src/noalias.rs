//! Раздел 4.5: что ссылки сообщают оптимизатору.
//!
//! Вызвать версию со ссылками так, чтобы счётчик лежал внутри данных,
//! нельзя:
//!
//! ```compile_fail,E0502
//! use ch04::noalias::ch04_count_flags;
//!
//! let mut words = [0x7E7E_7E7E_u32, 0x7D];
//! let bytes: &[u8] = bytemuck_free_view(&words);
//! ch04_count_flags(&mut words[1], bytes);
//! # fn bytemuck_free_view(w: &[u32; 2]) -> &[u8] {
//! #     unsafe { core::slice::from_raw_parts(w.as_ptr().cast(), 8) }
//! # }
//! ```

use core::cell::Cell;
use core::marker::PhantomPinned;

// ANCHOR: count
/// Сколько байтов-флагов 0x7E в принятом потоке. Счётчик — `&mut u32`,
/// данные — `&[u8]`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch04_count_flags(cnt: &mut u32, data: &[u8]) {
    for &b in data {
        *cnt += (b == 0x7E) as u32;
    }
}

/// То же на сырых указателях, как в C.
///
/// # Safety
///
/// `cnt` указывает на `u32`, доступный для чтения и записи, `data` — на
/// `n` байт, доступных для чтения. Пересекаться они могут.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn ch04_count_flags_raw(cnt: *mut u32, data: *const u8, n: usize) {
    for i in 0..n {
        // SAFETY: условия функции. Запись через `cnt` может изменить байты
        // `data`, и компилятор обязан это учитывать.
        unsafe { *cnt += (*data.add(i) == 0x7E) as u32 };
    }
}
// ANCHOR_END: count

// ANCHOR: kinds
/// `&T` без внутренней изменяемости: `noalias`, пока ссылка жива, значение
/// не меняется.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch04_na_shared(a: &u32) -> u32 {
    *a + 1
}

/// `&Cell<T>` меняет значение через разделяемую ссылку: `noalias` нет.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch04_na_cell(a: &Cell<u32>) -> u32 {
    a.get() + 2
}

/// Тип, который нельзя перемещать после закрепления (задача 3.D):
/// `&mut` на него тоже без `noalias`.
pub struct Pinned {
    pub x: u32,
    _pin: PhantomPinned,
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch04_na_pinned(a: &mut Pinned) -> u32 {
    a.x + 3
}
// ANCHOR_END: kinds

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_answer_without_overlap() {
        let data = [0x7E, 1, 0x7E, 0x7E, 2, 0x7E, 0x7E, 0x7E, 0, 0x7E];
        let mut a = 5;
        ch04_count_flags(&mut a, &data);
        let mut b = 5;
        // SAFETY: счётчик и данные — разные переменные.
        unsafe { ch04_count_flags_raw(&mut b, data.as_ptr(), data.len()) };
        assert_eq!((a, b), (12, 12));
    }

    #[test]
    fn kinds() {
        let c = Cell::new(1);
        let mut p = Pinned { x: 1, _pin: PhantomPinned };
        assert_eq!(ch04_na_shared(&1) + ch04_na_cell(&c) + ch04_na_pinned(&mut p), 9);
    }

    // ANCHOR: overlap
    /// Сырые указатели могут пересекаться, и тогда запись счётчика меняет
    /// данные, которые ещё предстоит прочитать.
    #[test]
    fn raw_pointers_may_overlap() {
        let mut words = [0x0000_007E_u32, 0x0000_007D];
        let base = words.as_mut_ptr();
        // SAFETY: оба указателя выведены из одного сырого указателя на
        // массив и не выходят за его границы; ссылок на массив нет.
        unsafe {
            let cnt = base.add(1); // второе слово
            let data = base.cast::<u8>(); // все 8 байт массива
            ch04_count_flags_raw(cnt, data, 8);
        }
        // Флаг в первом слове поднял счётчик с 0x7D до 0x7E. Младший байт
        // счётчика — тоже часть данных, и цикл, дойдя до него, насчитал
        // ещё один флаг.
        assert_eq!(words[1], 0x7F);
    }
    // ANCHOR_END: overlap

    // ANCHOR: overlap_ub
    /// То же пересечение через ссылки — неопределённое поведение: `&mut u32`
    /// и `&[u8]` на одни байты существовать одновременно не могут.
    #[test]
    #[ignore = "UB: запускать под Miri"]
    fn overlapping_references_are_ub() {
        let mut words = [0x0000_007E_u32, 0x0000_007D];
        let base = words.as_mut_ptr();
        // SAFETY: нет. Ссылки ниже пересекаются, это и показывает тест.
        unsafe {
            let cnt = &mut *base.add(1);
            let data = core::slice::from_raw_parts(base.cast::<u8>(), 8);
            ch04_count_flags(cnt, data);
        }
    }
    // ANCHOR_END: overlap_ub
}
