//! ЗАГОТОВКА аллокатора-счётчика. `Counted` уже передаёт все вызовы
//! системному распределителю, поэтому программа работает, но ничего не
//! считает. Ваша задача — добавить атомарные счётчики (один набор на
//! процесс) и заполнить [`counts`].
//!
//! Подсказки:
//! - счётчики — `static AtomicU64`/`AtomicUsize` с `Ordering::Relaxed`;
//! - `realloc` — это и освобождение старого блока, и выделение нового;
//! - пик занятой памяти удобно обновлять через `fetch_max` по значению,
//!   которое вернул `fetch_add`.

// Пока ничего не считаем — поля и импорты появятся вместе с решением.
#![allow(unused_imports, unused_variables)]

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::alloc::{GlobalAlloc, Layout, System};

/// Снимок счётчиков распределителя.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub allocs: u64,
    pub deallocs: u64,
    pub live: usize,
    pub peak: usize,
}

/// Текущее состояние счётчиков.
pub fn counts() -> Counts {
    // TODO: вернуть значения ваших атомиков вместо нулей.
    Counts::default()
}

pub struct Counted;

// SAFETY: каждый вызов передаётся System с теми же аргументами.
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: контракт alloc тот же, что у вызывающего.
        let p = unsafe { System.alloc(layout) };
        // TODO: если p не null — учесть выделение и байты.
        p
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: контракт realloc тот же, что у вызывающего.
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        // TODO: учесть и освобождение старого блока, и выделение нового.
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) };
        // TODO: учесть освобождение и уменьшить занятое.
    }
}
