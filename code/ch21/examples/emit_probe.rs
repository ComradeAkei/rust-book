//! Драйвер для теста кодогенерации (`tests/asm.rs`): вызывает атомарные
//! операции, чтобы их инструкции попали в объектный код. На x86-64 SeqCst-
//! store превращается в `xchg` (полный барьер), а RMW — в инструкции с
//! префиксом `lock`. `black_box` не даёт свернуть вызовы.

use std::hint::black_box;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[inline(never)]
fn seqcst_store(a: &AtomicU64, v: u64) {
    a.store(v, Ordering::SeqCst); // x86-64: xchg
}

#[inline(never)]
fn rmw_add(a: &AtomicUsize) -> usize {
    a.fetch_add(1, Ordering::AcqRel) // x86-64: lock xadd
}

#[inline(never)]
fn cas(a: &AtomicUsize) -> bool {
    // x86-64: lock cmpxchg
    a.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_ok()
}

fn main() {
    let a = AtomicU64::new(0);
    seqcst_store(&a, black_box(7));
    black_box(a.load(Ordering::SeqCst));

    let b = AtomicUsize::new(0);
    black_box(rmw_add(&b));
    black_box(cas(&b));
}
