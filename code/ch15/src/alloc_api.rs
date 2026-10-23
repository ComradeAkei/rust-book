//! Раздел 15.6: распределитель памяти. `Layout` — размер и выравнивание
//! блока, `alloc` и `dealloc` вручную, как `malloc` и `free`, и
//! распределитель-трассировщик, который ставится на место системного через
//! `#[global_allocator]`.

use alloc::alloc::{Layout, LayoutError, alloc, dealloc, handle_alloc_error};
use alloc::vec::Vec;

// ANCHOR: repr_c
/// Раскладка `repr(C)` по правилам C: поля по порядку, каждое — на
/// ближайшем смещении, кратном его выравниванию; в конце — дополнение до
/// выравнивания всей структуры.
pub fn repr_c(fields: &[Layout]) -> Result<(Layout, Vec<usize>), LayoutError> {
    let mut layout = Layout::new::<()>();
    let mut offsets = Vec::with_capacity(fields.len());
    for &f in fields {
        let (grown, offset) = layout.extend(f)?;
        layout = grown;
        offsets.push(offset);
    }
    Ok((layout.pad_to_align(), offsets))
}
// ANCHOR_END: repr_c

// ANCHOR: manual
/// Массив из `n` чисел через распределитель напрямую. В отличие от `free`,
/// `dealloc` получает размер: распределителю не обязательно хранить его в
/// заголовке блока.
pub fn squares_sum(n: usize) -> u64 {
    if n == 0 {
        return 0; // блок нулевого размера просить нельзя
    }
    let layout = Layout::array::<u64>(n).expect("переполнение размера");
    // SAFETY: размер layout не нулевой.
    let p = unsafe { alloc(layout) }.cast::<u64>();
    if p.is_null() {
        handle_alloc_error(layout); // аварийная остановка, как при неудаче Vec
    }
    let mut sum = 0;
    for i in 0..n {
        // SAFETY: i < n, блок выровнен под u64; запись до чтения —
        // память из alloc не инициализирована.
        unsafe {
            p.add(i).write((i * i) as u64);
            sum += p.add(i).read();
        }
    }
    // SAFETY: p выделен alloc с тем же layout и больше не используется.
    unsafe { dealloc(p.cast(), layout) };
    sum
}
// ANCHOR_END: manual

#[cfg(feature = "std")]
pub use trace::{Event, Trace};

#[cfg(feature = "std")]
mod trace {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

    // ANCHOR: trace
    /// Обращение к распределителю: размер и выравнивание блока, у `Realloc`
    /// ещё новый размер.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Event {
        Alloc { size: usize, align: usize },
        Zeroed { size: usize, align: usize },
        Realloc { size: usize, new_size: usize, align: usize },
        Dealloc { size: usize, align: usize },
    }

    const CAP: usize = 256;

    /// Распределитель, который записывает обращения одного потока к
    /// системному распределителю в массив фиксированного размера. Сам он
    /// память не выделяет: иначе каждая запись вызывала бы его же.
    pub struct Trace {
        events: [[AtomicUsize; 4]; CAP],
        len: AtomicUsize,
        lock: Mutex<()>,
    }

    thread_local! {
        // const и без Drop: доступ не выделяет память и работает даже при
        // завершении потока.
        static ON: Cell<bool> = const { Cell::new(false) };
    }
    // ANCHOR_END: trace

    impl Default for Trace {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Trace {
        pub const fn new() -> Self {
            Trace {
                events: [const { [const { AtomicUsize::new(0) }; 4] }; CAP],
                len: AtomicUsize::new(0),
                lock: Mutex::new(()),
            }
        }

        fn push(&self, kind: usize, size: usize, new_size: usize, align: usize) {
            if !ON.with(Cell::get) {
                return;
            }
            let i = self.len.fetch_add(1, Relaxed);
            if let Some(slot) = self.events.get(i) {
                for (cell, v) in slot.iter().zip([kind, size, new_size, align]) {
                    cell.store(v, Relaxed);
                }
            }
        }

        // ANCHOR: record
        /// Обращения к распределителю, которые сделала `f` в этом потоке.
        /// Записи сверх 256 теряются; `None`, если их было больше.
        pub fn record<R>(&self, f: impl FnOnce() -> R) -> (R, Option<Vec<Event>>) {
            let _one_at_a_time = self.lock.lock().unwrap_or_else(|e| e.into_inner());
            self.len.store(0, Relaxed);
            ON.with(|on| on.set(true));
            let r = f();
            ON.with(|on| on.set(false));
            let n = self.len.load(Relaxed);
            let events = (n <= CAP).then(|| {
                self.events[..n]
                    .iter()
                    .map(|e| {
                        let [k, size, new_size, align] = e.each_ref().map(|c| c.load(Relaxed));
                        match k {
                            0 => Event::Alloc { size, align },
                            1 => Event::Zeroed { size, align },
                            2 => Event::Realloc { size, new_size, align },
                            _ => Event::Dealloc { size, align },
                        }
                    })
                    .collect()
            });
            (r, events)
        }
        // ANCHOR_END: record
    }

    // ANCHOR: impl
    // SAFETY: все вызовы передаются System как есть; запись в массив не
    // трогает выделенную память.
    unsafe impl GlobalAlloc for Trace {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            self.push(0, layout.size(), 0, layout.align());
            // SAFETY: контракт alloc тот же, что у вызывающего.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            self.push(1, layout.size(), 0, layout.align());
            // SAFETY: контракт alloc_zeroed тот же, что у вызывающего.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            self.push(2, layout.size(), new_size, layout.align());
            // SAFETY: контракт realloc тот же, что у вызывающего.
            unsafe { System.realloc(ptr, layout, new_size) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            self.push(3, layout.size(), 0, layout.align());
            // SAFETY: ptr выделен System с тем же layout.
            unsafe { System.dealloc(ptr, layout) }
        }
    }
    // ANCHOR_END: impl
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::offset_of;

    #[repr(C)]
    struct Header {
        kind: u8,
        timestamp: u32,
        sensor: u16,
    }

    #[test]
    fn repr_c_matches_compiler() {
        let (l, offs) =
            repr_c(&[Layout::new::<u8>(), Layout::new::<u32>(), Layout::new::<u16>()]).unwrap();
        assert_eq!(
            offs,
            [offset_of!(Header, kind), offset_of!(Header, timestamp), offset_of!(Header, sensor)]
        );
        assert_eq!(offs, [0, 4, 8]);
        assert_eq!(l, Layout::new::<Header>());
        assert_eq!((l.size(), l.align()), (12, 4));
    }

    #[test]
    fn manual_array() {
        assert_eq!(squares_sum(0), 0);
        assert_eq!(squares_sum(4), 1 + 4 + 9);
    }

    #[test]
    fn layout_rejects_overflow() {
        assert!(Layout::array::<u64>(usize::MAX / 4).is_err());
        assert!(Layout::from_size_align(8, 3).is_err());
    }
}
