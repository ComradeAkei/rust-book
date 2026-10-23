//! Задача D: свой тип динамического размера. `Record<T>` держит заголовок
//! и хвост `[T]` переменной длины в ОДНОМ выделении; `TwoBox<T>` хранит те
//! же данные, но срез — в отдельном блоке кучи. Корректность `Record`
//! держится на двух вещах: раскладка ручного выделения совпадает с той, что
//! выведет компилятор (иначе `Box` освободит не так), и элементы `[T]`
//! роняет автоматический drop-glue `Box<Record>` — своего `Drop` писать не
//! нужно, иначе получилось бы двойное освобождение.

use std::alloc::{Layout, alloc, handle_alloc_error};
use std::ptr;

// ANCHOR: record
#[repr(C)]
pub struct Record<T> {
    pub tag: u32,
    pub data: [T],
}

impl<T> Record<T> {
    /// Собрать `Box<Record<T>>` с `data = items` в одном выделении,
    /// переместив элементы без копий значений.
    pub fn new(tag: u32, items: Vec<T>) -> Box<Record<T>> {
        let n = items.len();
        let (layout, data_off) = Self::layout(n);
        let mut items = items;
        // SAFETY: layout ненулевого размера (минимум заголовок u32). Пишем
        // tag в начало, переносим n элементов в хвост побайтно; set_len(0)
        // не даёт Vec уронить перемещённые элементы (освободит лишь буфер).
        // Толстый указатель получает адрес начала записи и длину хвоста n —
        // ровно то, что drop-glue Box использует для Drop и освобождения.
        unsafe {
            let thin = alloc(layout);
            if thin.is_null() {
                handle_alloc_error(layout);
            }
            (thin as *mut u32).write(tag);
            ptr::copy_nonoverlapping(items.as_ptr(), thin.add(data_off) as *mut T, n);
            items.set_len(0);
            let fat = ptr::slice_from_raw_parts_mut(thin as *mut T, n) as *mut Record<T>;
            Box::from_raw(fat)
        }
    }

    /// Раскладка `Record<T>` с хвостом длины `n` и смещение хвоста — та же,
    /// что выведет компилятор для `#[repr(C)]`.
    fn layout(n: usize) -> (Layout, usize) {
        let header = Layout::new::<u32>();
        let tail = Layout::array::<T>(n).unwrap();
        let (layout, off) = header.extend(tail).unwrap();
        (layout.pad_to_align(), off)
    }
}
// ANCHOR_END: record

// ANCHOR: twobox
/// Тот же смысл, но заголовок и срез — в разных местах: `Box<[T]>` — это
/// отдельное выделение, а `TwoBox` хранит толстый указатель на него плюс
/// `tag`. Данных столько же, но на куче два блока вместо одного и лишнее
/// разыменование к хвосту.
pub struct TwoBox<T> {
    pub tag: u32,
    pub data: Box<[T]>,
}

impl<T> TwoBox<T> {
    pub fn new(tag: u32, items: Vec<T>) -> Self {
        TwoBox { tag, data: items.into_boxed_slice() }
    }
}
// ANCHOR_END: twobox
