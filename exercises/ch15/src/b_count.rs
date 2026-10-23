//! Задача B: распределитель-счётчик. Ставится через `#[global_allocator]`,
//! передаёт все вызовы системному распределителю и считает обращения,
//! выделенные байты и пик занятой памяти.
//!
//! Заготовка уже передаёт вызовы `System`, но ничего не считает, а
//! `measure` — `todo!()`. Тесты идут параллельно в разных потоках и все
//! через этот распределитель: `measure` должен видеть только свой поток.
//! Помните, что распределитель зовут отовсюду, в том числе из кода,
//! который вы напишете для подсчёта, и пока поток завершается.

use std::alloc::{GlobalAlloc, Layout, System};

// ANCHOR: stats
/// Что сделала измеренная функция. Байты — размеры из `Layout`, без
/// заголовков блоков, которые добавляет системный распределитель.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Вызовы `alloc` и `alloc_zeroed`.
    pub allocs: u64,
    pub reallocs: u64,
    pub deallocs: u64,
    /// Сколько байт запрошено всего: новые блоки и прирост при `realloc`.
    pub requested: u64,
    /// Наибольший прирост занятой памяти над уровнем на входе.
    pub peak: u64,
    /// Насколько больше памяти занято на выходе, чем на входе: результат
    /// функции и утечки. Отрицательно, если функция освободила чужое.
    pub retained: i64,
}
// ANCHOR_END: stats

pub struct Counting;

// SAFETY: все вызовы передаются System как есть.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: контракт alloc тот же, что у вызывающего.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: контракт alloc_zeroed тот же, что у вызывающего.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: контракт realloc тот же, что у вызывающего.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// Выполнить `f` и вернуть её результат вместе с обращениями к
/// распределителю, которые она сделала в этом потоке. Вложенные вызовы
/// допустимы: внешний видит пик внутреннего.
pub fn measure<R>(f: impl FnOnce() -> R) -> (R, Stats) {
    todo!()
}
