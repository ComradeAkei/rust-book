//! Аллокатор-счётчик для всего процесса. В отличие от распределителя
//! задачи 15.B, который считает в каждом потоке отдельно через
//! thread-local, этот ведёт один счётчик на процесс в атомиках: хранилище
//! наполняет один поток, а проверить «в установившемся режиме куча не
//! растёт» можно и не привязываясь к потоку.
//!
//! Атомики с `Relaxed`: нам нужна не синхронизация с чужой памятью, а
//! только то, чтобы сами счётчики не теряли инкрементов. Байты считаются по
//! `Layout::size`, без заголовков блоков самого распределителя.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::alloc::{GlobalAlloc, Layout, System};

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static DEALLOCS: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// ANCHOR: counts
/// Снимок счётчиков распределителя. `allocs` и `deallocs` — число вызовов
/// (`realloc` считается и как освобождение старого блока, и как выделение
/// нового), `live` — сколько байт занято сейчас, `peak` — наибольшее
/// занятое за всё время.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub allocs: u64,
    pub deallocs: u64,
    pub live: usize,
    pub peak: usize,
}

/// Текущее состояние счётчиков.
pub fn counts() -> Counts {
    Counts {
        allocs: ALLOCS.load(Relaxed),
        deallocs: DEALLOCS.load(Relaxed),
        live: LIVE.load(Relaxed),
        peak: PEAK.load(Relaxed),
    }
}
// ANCHOR_END: counts

// ANCHOR: bump
fn bump_live(delta: usize) {
    // fetch_add возвращает прежнее значение; обновляем пик по нему.
    let before = LIVE.fetch_add(delta, Relaxed);
    PEAK.fetch_max(before + delta, Relaxed);
}
// ANCHOR_END: bump

// ANCHOR: counted
pub struct Counted;

// SAFETY: каждый вызов передаётся System с теми же аргументами; счётчики —
// атомики, они не выделяют память и не трогают выданные блоки.
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: контракт alloc тот же, что у вызывающего.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            ALLOCS.fetch_add(1, Relaxed);
            bump_live(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: контракт alloc_zeroed тот же, что у вызывающего.
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            ALLOCS.fetch_add(1, Relaxed);
            bump_live(layout.size());
        }
        p
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: контракт realloc тот же, что у вызывающего.
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            ALLOCS.fetch_add(1, Relaxed);
            DEALLOCS.fetch_add(1, Relaxed);
            // Занятое меняется на разницу размеров; пик учитываем, только
            // если блок вырос.
            if new_size >= layout.size() {
                bump_live(new_size - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Relaxed);
            }
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) };
        DEALLOCS.fetch_add(1, Relaxed);
        LIVE.fetch_sub(layout.size(), Relaxed);
    }
}
// ANCHOR_END: counted
