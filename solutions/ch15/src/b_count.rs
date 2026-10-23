//! Задача B: распределитель-счётчик. Ставится через `#[global_allocator]`,
//! передаёт все вызовы системному распределителю и считает в каждом потоке
//! обращения, выделенные байты и пик занятой памяти. `measure` отвечает на
//! вопрос «сколько стоила эта функция» и не путает её с соседними
//! потоками, например с другими тестами.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

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

// ANCHOR: counters
/// Счётчики потока. `const` и типы без `Drop`: доступ к ним не выделяет
/// память и не регистрирует деструктор, поэтому его можно звать из
/// распределителя, в том числе пока поток завершается.
#[derive(Clone, Copy)]
struct Counters {
    allocs: u64,
    reallocs: u64,
    deallocs: u64,
    requested: u64,
    live: i64,
    peak: i64,
}

thread_local! {
    static C: Cell<Counters> = const {
        Cell::new(Counters { allocs: 0, reallocs: 0, deallocs: 0, requested: 0, live: 0, peak: 0 })
    };
}

fn update(f: impl FnOnce(&mut Counters)) {
    // try_with: на некоторых платформах к TLS нельзя обратиться после
    // уничтожения потока; такие вызовы просто не считаются.
    let _ = C.try_with(|c| {
        let mut v = c.get();
        f(&mut v);
        v.peak = v.peak.max(v.live);
        c.set(v);
    });
}
// ANCHOR_END: counters

// ANCHOR: alloc
pub struct Counting;

// SAFETY: все вызовы передаются System как есть; счётчики не трогают
// выделенную память и сами её не выделяют.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: контракт alloc тот же, что у вызывающего.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            update(|c| {
                c.allocs += 1;
                c.requested += layout.size() as u64;
                c.live += layout.size() as i64;
            });
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: контракт alloc_zeroed тот же, что у вызывающего.
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            update(|c| {
                c.allocs += 1;
                c.requested += layout.size() as u64;
                c.live += layout.size() as i64;
            });
        }
        p
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: контракт realloc тот же, что у вызывающего.
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        // При неудаче старый блок цел, и ничего не изменилось.
        if !p.is_null() {
            let delta = new_size as i64 - layout.size() as i64;
            update(|c| {
                c.reallocs += 1;
                c.requested += delta.max(0) as u64;
                c.live += delta;
            });
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) };
        update(|c| {
            c.deallocs += 1;
            c.live -= layout.size() as i64;
        });
    }
}
// ANCHOR_END: alloc

// ANCHOR: measure
/// Выполнить `f` и вернуть её результат вместе с обращениями к
/// распределителю, которые она сделала в этом потоке. Вложенные вызовы
/// допустимы: внешний видит пик внутреннего.
pub fn measure<R>(f: impl FnOnce() -> R) -> (R, Stats) {
    let before = C.with(Cell::get);
    // Пик отсчитывается заново от текущего уровня, внешний — сохраняется.
    C.with(|c| c.set(Counters { peak: before.live, ..before }));
    let r = f();
    let after = C.with(Cell::get);
    C.with(|c| c.set(Counters { peak: before.peak.max(after.peak), ..after }));
    let stats = Stats {
        allocs: after.allocs - before.allocs,
        reallocs: after.reallocs - before.reallocs,
        deallocs: after.deallocs - before.deallocs,
        requested: after.requested - before.requested,
        peak: (after.peak - before.live) as u64,
        retained: after.live - before.live,
    };
    (r, stats)
}
// ANCHOR_END: measure
