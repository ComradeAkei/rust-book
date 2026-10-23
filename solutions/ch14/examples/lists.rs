//! Задача C: два двусвязных списка на одной и той же работе — построить
//! список из N узлов, обойти его, переставить в начало M случайных узлов
//! по ручкам, как кэш LRU, и снова обойти. Печатает время на операцию и
//! байты кучи на узел.
//!
//! Запуск: `cargo run --release -p ch14-solutions --example lists [rc|arena] [N]`.
//! Без первого аргумента — оба списка, каждый в своём процессе. Промахи кэша:
//! `valgrind --tool=cachegrind --cache-sim=yes target/release/examples/lists rc 100000`.
use ch14_solutions::c_list::{ArenaList, RcList};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

/// Распределитель, который помнит занятые байты и их максимум.
struct Peak;

static NOW: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: все вызовы передаются системному распределителю как есть.
unsafe impl GlobalAlloc for Peak {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let now = NOW.fetch_add(layout.size(), Relaxed) + layout.size();
        PEAK.fetch_max(now, Relaxed);
        // SAFETY: контракт alloc тот же, что у вызывающего.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        NOW.fetch_sub(layout.size(), Relaxed);
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) }
    }

    /// Через системный realloc: большие блоки glibc переносит mremap, без
    /// копирования.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let now = NOW.fetch_add(new_size, Relaxed) + new_size;
        PEAK.fetch_max(now, Relaxed);
        NOW.fetch_sub(layout.size(), Relaxed);
        // SAFETY: контракт realloc тот же, что у вызывающего.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static PEAK_ALLOC: Peak = Peak;

/// Псевдослучайные индексы без зависимости от крейтов.
fn indices(n: usize, m: usize) -> Vec<usize> {
    let mut s = 0x9E37_79B9u32;
    (0..m)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as usize % n
        })
        .collect()
}

fn ns(t: Instant, ops: usize) -> f64 {
    t.elapsed().as_secs_f64() * 1e9 / ops as f64
}

macro_rules! run {
    ($name:literal, $list:expr, $n:expr, $touch:expr, $front:expr) => {{
        let (n, touch): (usize, &[usize]) = ($n, $touch);
        let base = NOW.load(Relaxed);
        PEAK.store(base, Relaxed);
        let t = Instant::now();
        let mut list = $list;
        let handles: Vec<_> = (0..n as u64).map(|i| list.push_back(i)).collect();
        let build = ns(t, n);
        let handle_bytes = handles.capacity() * size_of_val(&handles[0]);
        let bytes = (NOW.load(Relaxed) - base - handle_bytes) as f64 / n as f64;
        let t = Instant::now();
        black_box(list.to_vec());
        let walk = ns(t, n);
        let t = Instant::now();
        for &i in touch {
            assert!($front(&mut list, black_box(&handles[i])));
        }
        let lru = ns(t, touch.len());
        let t = Instant::now();
        let v = list.to_vec();
        let walk2 = ns(t, n);
        assert_eq!(v.len(), n);
        println!(
            "{:6} постройка {build:5.1} нс, обход {walk:5.1} нс, в начало {lru:5.1} нс, \
             обход после {walk2:5.1} нс, {bytes:5.1} байт на узел",
            $name
        );
    }};
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    let n: usize = std::env::args().nth(2).map_or(1_000_000, |s| s.parse().unwrap());
    let touch = indices(n, n);
    match which.as_str() {
        "rc" => run!("Rc", RcList::new(), n, &touch, |l: &mut RcList<u64>, h| l.move_to_front(h)),
        "arena" => {
            run!("арена", ArenaList::new(), n, &touch, |l: &mut ArenaList<u64>, h: &_| {
                l.move_to_front(*h)
            })
        }
        // Каждый список — в своём процессе: освобождённые узлы первого
        // меняют поведение распределителя для второго.
        _ => {
            for w in ["rc", "arena"] {
                let me = std::env::current_exe().unwrap();
                let st = std::process::Command::new(me).args([w, &n.to_string()]).status();
                assert!(st.unwrap().success());
            }
        }
    }
}
