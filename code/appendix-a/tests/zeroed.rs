//! Раздел A.3: `vec![0; n]` просит у аллокатора уже обнулённую память
//! (`alloc_zeroed`, в System это `calloc`), а не `malloc` с `memset`.
//! Это наблюдение о текущей стандартной библиотеке, а не гарантия языка.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

struct Counting;

static ZEROED: AtomicUsize = AtomicUsize::new(0);

// SAFETY: все вызовы передаются System без изменений, контракт
// GlobalAlloc выполняет System; счётчик на выделение памяти не влияет.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: тот же контракт, что у вызывающего.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ZEROED.fetch_add(1, Relaxed);
        // SAFETY: тот же контракт, что у вызывающего.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` выделен через System с этим же `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

#[test]
fn vec_of_zeros_is_calloc() {
    let before = ZEROED.load(Relaxed);
    let zeros = vec![0u8; 1 << 16];
    assert_eq!(ZEROED.load(Relaxed), before + 1);

    let ones = vec![1u8; 1 << 16]; // malloc и заполнение
    assert_eq!(ZEROED.load(Relaxed), before + 1);
    assert_eq!(zeros.len() + ones.len(), 1 << 17);
}
